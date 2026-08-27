//! Same integration test as `pantheon_conversation.rs`, redesigned per
//! Jason's follow-up: "what if we seeded a conversation with 4 different
//! GLM personalities (Athena, Artemis, Apollo, Dionysus), and used
//! structured-output to discipline the response, e.g. reply only in
//! DMML?"
//!
//! Two real design changes from the first run:
//!
//! 1. **One model, four personas** -- all four Olympians are
//!    `z-ai/glm-5.3`, distinguished only by system prompt. This isolates
//!    whether persona framing alone produces genuinely distinguishable
//!    philosophical moves, separate from the model-capability
//!    differences that confounded the first run (kimi/deepseek's
//!    citation-copying failures vs. glm's).
//! 2. **Tool-calling instead of prompt-requested JSON.** Tested live
//!    first, not assumed: `response_format: {"type":"json_schema",
//!    "strict":true}` is accepted by GLM-5.3's endpoint (it's listed in
//!    `supported_parameters`) but silently IGNORED -- two live test
//!    calls both returned free-form in-character prose, no JSON at all,
//!    `finish_reason: stop`. Z.AI's backend also rejects a *forced*
//!    tool_choice (`400: "Tool choice must be auto"`), but with
//!    `tool_choice: "auto"` plus an explicit instruction to call the
//!    tool, it reliably emits a real, schema-valid `submit_dmml_turn`
//!    call. That's the actual discipline mechanism this file uses --
//!    real, verified working, not the one first asked for.
//!
//! Updated per Jason's follow-up: dispatch now goes through
//! `async-openai` (pointed at OpenRouter's own OpenAI-compatible
//! `/v1` base -- confirmed this crate's typed `reasoning_effort` field
//! round-trips correctly against GLM-5.3 via a live test call, same
//! effect as the raw `{"reasoning":{"effort":...}}` object form used
//! before) instead of hand-rolled `reqwest` JSON, and five rounds
//! instead of two.

use std::collections::HashMap;

use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTool, ChatCompletionToolChoiceOption,
    ChatCompletionTools, CreateChatCompletionRequestArgs, FunctionObject, ReasoningEffort,
    ToolChoiceOptions,
};
use async_openai::Client;
use dmml_runtime::graph::{Commit, ConsumeRef, FactRef, StrongRef};
use dmml_runtime::substrate::AppendSubstrate;
use dmml_substrate_kit::iroh_substrate::IrohAppendSubstrate;
use iroh::endpoint::Builder as EndpointBuilder;
use iroh_blobs::store::mem::MemStore;
use iroh_docs::api::Doc;
use iroh_docs::protocol::Docs;
use iroh_docs::AuthorId;
use serde::{Deserialize, Serialize};

struct Anchor {
    rkey: &'static str,
    verb: &'static str,
    subject: &'static str,
    predicate: &'static str,
    object: &'static str,
}

// Same eight real claims as pantheon_conversation.rs, pulled verbatim
// from benjamin_full_essay.rs's own graph.
const ANCHORS: &[Anchor] = &[
    Anchor { rkey: "rkey0008", verb: "coins", subject: "argument/section_ii", predicate: "claim", object: "aura names the authenticity-testimony-authority chain" },
    Anchor { rkey: "rkey0011", verb: "stipulates", subject: "argument/section_iii_aura_natural", predicate: "naturalAuraDefinition", object: "unique phenomenon of a distance, however close it may be" },
    Anchor { rkey: "rkey0018", verb: "argues", subject: "argument/section_v", predicate: "qualitativeShift", object: "the quantitative shift between cult and exhibition value turned into a qualitative transformation of art's nature" },
    Anchor { rkey: "rkey0026", verb: "reproduces", subject: "argument/section_x_star_cult", predicate: "claim", object: "the cult of the movie star preserves not the unique aura of the person but the phony spell of a commodity" },
    Anchor { rkey: "rkey0029", verb: "argues", subject: "role/magician", predicate: "structuralMatch", object: "the magician's authority-based distance instantiates aura-as-distance, transposed onto medicine" },
    Anchor { rkey: "rkey0030", verb: "argues", subject: "artist/cameraman", predicate: "pictureType", object: "multiple fragments assembled under a new law, matching the surgeon's structure" },
    Anchor { rkey: "rkey0040", verb: "asserts", subject: "argument/epilogue_aestheticize", predicate: "claim", object: "Fascism gives the masses expression while preserving property -- the introduction of aesthetics into political life" },
    Anchor { rkey: "rkey0044", verb: "argues", subject: "argument/epilogue", predicate: "claim", object: "Fascism is rendering politics aesthetic. Communism responds by politicizing art." },
];

struct Olympian {
    name: &'static str,
    persona: &'static str,
}

const OLYMPIANS: &[Olympian] = &[
    Olympian {
        name: "athena",
        persona: "You are Athena, goddess of wisdom and strategy. You argue \
carefully, look for the load-bearing structural claim underneath a surface \
disagreement, and favor precision over drama.",
    },
    Olympian {
        name: "artemis",
        persona: "You are Artemis, goddess of the hunt and fierce independence. \
You refuse to accept a claim just because it was stated confidently by \
another speaker; you go looking for what the conversation has overlooked \
or is too polite to say.",
    },
    Olympian {
        name: "apollo",
        persona: "You are Apollo, god of order, harmony, and prophecy. You look \
for the underlying pattern connecting claims that seem unrelated, and you \
favor claims that resolve tension into a clearer structure.",
    },
    Olympian {
        name: "dionysus",
        persona: "You are Dionysus, god of ecstasy, transgression, and the \
dissolution of fixed categories. You are suspicious of any claim that \
settles a question too neatly, and you push toward what a tidy resolution \
is quietly excluding.",
    },
];

const ROUNDS: usize = 5;
const MODEL: &str = "z-ai/glm-5.3";

#[derive(Debug, Clone)]
struct TurnRecord {
    cid: String,
    respondent: String,
    verb: String,
    subject: String,
    predicate: String,
    object: String,
    consumes: Vec<(String, String, String)>,
}

#[derive(Deserialize)]
struct DmmlTurnArgs {
    verb: String,
    subject: String,
    predicate: String,
    object: String,
    #[serde(default)]
    consumes: Vec<CitedFact>,
}

#[derive(Deserialize)]
struct CitedFact {
    cid: String,
    subject: String,
    predicate: String,
}

#[derive(Serialize)]
struct DumpedTurn<'a> {
    cid: &'a str,
    respondent: &'a str,
    verb: &'a str,
    subject: &'a str,
    predicate: &'a str,
    object: &'a str,
    consumes: &'a [(String, String, String)],
}

fn transcript_so_far(log: &[TurnRecord]) -> String {
    log.iter()
        .map(|t| {
            format!(
                "- cid={} respondent={} verb={} | {} {} \"{}\"",
                t.cid, t.respondent, t.verb, t.subject, t.predicate, t.object
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn dmml_turn_tool() -> ChatCompletionTools {
    ChatCompletionTools::Function(ChatCompletionTool {
        function: FunctionObject {
            name: "submit_dmml_turn".to_string(),
            description: Some("Submit exactly one DMML conversational turn.".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "verb": {"type": "string", "enum": ["argues", "questions", "extends", "disputes", "connects"]},
                    "subject": {"type": "string", "description": "short slug for what this turn is about"},
                    "predicate": {"type": "string", "description": "short camelCase predicate naming the claim"},
                    "object": {"type": "string", "description": "the actual claim, one or two sentences, in your own voice"},
                    "consumes": {
                        "type": "array",
                        "description": "at least one EXACT (cid, subject, predicate) copied from the log you were shown -- never invent one",
                        "items": {
                            "type": "object",
                            "properties": {
                                "cid": {"type": "string"},
                                "subject": {"type": "string"},
                                "predicate": {"type": "string"}
                            },
                            "required": ["cid", "subject", "predicate"]
                        }
                    }
                },
                "required": ["verb", "subject", "predicate", "object", "consumes"]
            })),
            strict: None,
        },
    })
}

async fn dispatch(
    client: &Client<OpenAIConfig>,
    olympian: &Olympian,
    log: &[TurnRecord],
) -> anyhow::Result<DmmlTurnArgs> {
    let user_msg = format!(
        "Four Olympians (Athena, Artemis, Apollo, Dionysus) are analyzing Walter Benjamin's \
\"The Work of Art in the Age of Mechanical Reproduction\" together, in a real, growing, checkable \
DMML commit log. Some entries are Benjamin's own claims (respondent=essay); others are prior turns \
from you or the other three.\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona -- agreeing with, \
extending, disputing, or connecting something SPECIFIC already in the log, in a way another \
Olympian with your temperament actually would. Not a summary of Benjamin, not a restatement. If you \
don't have a real move to make, say so honestly in `object` rather than padding. Call \
submit_dmml_turn with your answer. `consumes` must copy at least one real (cid, subject, predicate) \
from the log above exactly -- never invent one.",
        transcript_so_far(log)
    );

    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .reasoning_effort(ReasoningEffort::Low)
        .max_completion_tokens(1500u32)
        .messages(vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(olympian.persona)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_msg)
                .build()?
                .into(),
        ])
        .tools(vec![dmml_turn_tool()])
        .tool_choice(ChatCompletionToolChoiceOption::Mode(ToolChoiceOptions::Auto))
        .build()?;

    let response = client.chat().create(request).await?;
    let message = &response
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("no choices in response"))?
        .message;
    let tool_calls = message
        .tool_calls
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no tool_calls in response message: {message:?}"))?;
    let ChatCompletionMessageToolCalls::Function(call) = tool_calls
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty tool_calls array"))?
    else {
        anyhow::bail!("first tool call was not a function call");
    };
    let args = &call.function.arguments;
    serde_json::from_str(args)
        .map_err(|e| anyhow::anyhow!("failed to parse tool arguments ({e}): {args}"))
}

fn nquad(subject_slug: &str, predicate: &str, object: &str) -> String {
    format!(
        "_:{subject_slug} <https://written-world.example/predicate/{predicate}> {} .",
        serde_json::to_string(object).unwrap()
    )
}

async fn append(
    substrate: &IrohAppendSubstrate,
    author: &AuthorId,
    verb: &str,
    subject: &str,
    predicate: &str,
    object: &str,
    consumes_facts: &[(String, String, String)],
) -> anyhow::Result<TurnRecord> {
    let consumes = consumes_facts
        .iter()
        .map(|(cid, subj, pred)| {
            ConsumeRef::Fact(FactRef {
                commit: StrongRef {
                    uri: format!("iroh://pantheon-olympians/{cid}"),
                    cid: cid.clone(),
                },
                subject: subj.clone(),
                predicate: pred.clone(),
                object: None,
            })
        })
        .collect();
    let slug = subject.replace(['/', ' '], "_");
    let commit = Commit {
        consumes,
        produces: nquad(&slug, predicate, object),
        predicate: verb.to_string(),
        via: None,
        responds_to: None,
        created_at: "2026-08-27T00:00:00Z".to_string(),
    };
    let receipt = substrate.append_commit(author, &commit).await?;
    Ok(TurnRecord {
        cid: receipt.cid,
        respondent: String::new(),
        verb: verb.to_string(),
        subject: subject.to_string(),
        predicate: predicate.to_string(),
        object: object.to_string(),
        consumes: consumes_facts.to_vec(),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("== pantheon olympians: 4 GLM-5.3 personas, tool-call-disciplined DMML ==\n");

    let endpoint = EndpointBuilder::empty()
        .crypto_provider(std::sync::Arc::new(rustls::crypto::ring::default_provider()))
        .bind()
        .await?;
    let blobs = MemStore::default();
    let gossip = iroh_gossip::net::Gossip::builder().spawn(endpoint.clone());
    let docs = Docs::memory()
        .spawn(endpoint.clone(), (*blobs).clone(), gossip.clone())
        .await?;
    let api = docs.api();
    let doc: Doc = api.create().await?;
    println!("doc namespace: {}\n", doc.id());

    let essay_author = api.author_create().await?;
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        essay_author,
        "pantheon-olympians".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} real anchor claims --", ANCHORS.len());
    for a in ANCHORS {
        let mut rec = append(&substrate, &essay_author, a.verb, a.subject, a.predicate, a.object, &[]).await?;
        rec.respondent = "essay".to_string();
        println!("  [{}] {} -> {} \"{}\"", a.rkey, rec.cid, rec.subject, rec.object);
        log.push(rec);
    }

    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set"))?;
    let client = Client::with_config(
        OpenAIConfig::new()
            .with_api_base("https://openrouter.ai/api/v1")
            .with_api_key(api_key),
    );

    for round in 1..=ROUNDS {
        println!("\n-- round {round} --");
        for olympian in OLYMPIANS {
            print!("  dispatching {}... ", olympian.name);
            use std::io::Write;
            std::io::stdout().flush().ok();
            match dispatch(&client, olympian, &log).await {
                Ok(reply) => {
                    let mut verified = Vec::new();
                    for c in &reply.consumes {
                        let real = log
                            .iter()
                            .any(|t| t.cid == c.cid && t.subject == c.subject && t.predicate == c.predicate);
                        if real {
                            verified.push((c.cid.clone(), c.subject.clone(), c.predicate.clone()));
                        } else {
                            println!(
                                "\n    [WARNING] {} cited a non-existent fact (cid={}, subject={}, predicate={}) -- dropped",
                                olympian.name, c.cid, c.subject, c.predicate
                            );
                        }
                    }
                    let author = olympian_authors[olympian.name];
                    let mut rec = append(
                        &substrate,
                        &author,
                        &reply.verb,
                        &reply.subject,
                        &reply.predicate,
                        &reply.object,
                        &verified,
                    )
                    .await?;
                    rec.respondent = olympian.name.to_string();
                    println!(
                        "ok -> {} : {} {} \"{}\" (consumes {})",
                        rec.cid, rec.subject, rec.predicate, rec.object, verified.len()
                    );
                    log.push(rec);
                }
                Err(e) => println!("FAILED: {e}"),
            }
        }
    }

    println!("\n-- final transcript: {} real entries --", log.len());
    let dumped: Vec<DumpedTurn> = log
        .iter()
        .map(|t| DumpedTurn {
            cid: &t.cid,
            respondent: &t.respondent,
            verb: &t.verb,
            subject: &t.subject,
            predicate: &t.predicate,
            object: &t.object,
            consumes: &t.consumes,
        })
        .collect();
    let json = serde_json::to_string_pretty(&dumped)?;
    std::fs::write("pantheon_olympians.json", &json)?;
    println!("wrote pantheon_olympians.json ({} entries)", dumped.len());

    Ok(())
}
