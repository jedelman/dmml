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
//!
//! Updated again per Jason's next follow-up: the single-source-material
//! run "may not have anything to add" partly because it only had one
//! source to react to. Anchors now seed from TWO real, tension-bearing
//! sources -- Benjamin's essay and Adorno's actual, documented direct
//! critique of that same essay (his 18 March 1936 letter, plus Adorno &
//! Horkheimer's "The Culture Industry") -- both verified against real
//! quotes (see the `ANCHORS` doc comment for exact sourcing), never
//! invented, same discipline as the Benjamin anchors always used. The
//! four Olympians are told both sources exist and are given explicit
//! autonomy to interpret, counter-interpret, or synthesize across them.

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
    id: &'static str,
    author: &'static str, // "benjamin" or "adorno" -- a real, distinct source identity
    verb: &'static str,
    subject: &'static str,
    predicate: &'static str,
    object: &'static str,
}

// Benjamin's eight, same as before, pulled verbatim from
// benjamin_full_essay.rs's own graph.
//
// Adorno's eight are new, per Jason's follow-up ("bring in both texts;
// the dialogue agents should have autonomy to interpret and
// counter-interpret them") -- real, verified quotes, not paraphrased or
// invented, from two real sources:
//
// - Adorno's actual 18 March 1936 letter to Benjamin, a real, documented
//   direct critique of this exact essay (Benjamin never fully answered
//   it). Verified via https://classicalmusiclife.substack.com/p/theodor-adorno-to-walter-benjamin
//   (secondary source quoting the letter directly) after a primary-source
//   PDF (platypus1917.org) turned out to be an unreadable scanned image,
//   not usable text.
// - Adorno & Horkheimer's "The Culture Industry: Enlightenment as Mass
//   Deception" (Dialectic of Enlightenment, 1944) -- verified against the
//   full text at https://www.marxists.org/reference/archive/adorno/1944/culture-industry.htm
const ANCHORS: &[Anchor] = &[
    Anchor { id: "benjamin/rkey0008", author: "benjamin", verb: "coins", subject: "argument/section_ii", predicate: "claim", object: "aura names the authenticity-testimony-authority chain" },
    Anchor { id: "benjamin/rkey0011", author: "benjamin", verb: "stipulates", subject: "argument/section_iii_aura_natural", predicate: "naturalAuraDefinition", object: "unique phenomenon of a distance, however close it may be" },
    Anchor { id: "benjamin/rkey0018", author: "benjamin", verb: "argues", subject: "argument/section_v", predicate: "qualitativeShift", object: "the quantitative shift between cult and exhibition value turned into a qualitative transformation of art's nature" },
    Anchor { id: "benjamin/rkey0026", author: "benjamin", verb: "reproduces", subject: "argument/section_x_star_cult", predicate: "claim", object: "the cult of the movie star preserves not the unique aura of the person but the phony spell of a commodity" },
    Anchor { id: "benjamin/rkey0029", author: "benjamin", verb: "argues", subject: "role/magician", predicate: "structuralMatch", object: "the magician's authority-based distance instantiates aura-as-distance, transposed onto medicine" },
    Anchor { id: "benjamin/rkey0030", author: "benjamin", verb: "argues", subject: "artist/cameraman", predicate: "pictureType", object: "multiple fragments assembled under a new law, matching the surgeon's structure" },
    Anchor { id: "benjamin/rkey0040", author: "benjamin", verb: "asserts", subject: "argument/epilogue_aestheticize", predicate: "claim", object: "Fascism gives the masses expression while preserving property -- the introduction of aesthetics into political life" },
    Anchor { id: "benjamin/rkey0044", author: "benjamin", verb: "argues", subject: "argument/epilogue", predicate: "claim", object: "Fascism is rendering politics aesthetic. Communism responds by politicizing art." },
    // Adorno's 1936 letter to Benjamin -- direct critique of this essay.
    Anchor { id: "adorno/letter_myth", author: "adorno", verb: "disputes", subject: "letter/autonomous_art_myth", predicate: "claim", object: "the center of the autonomous work of art does not itself belong on the side of myth" },
    Anchor { id: "adorno/letter_laughter", author: "adorno", verb: "disputes", subject: "letter/cinema_laughter", predicate: "claim", object: "The laughter of the audience at a cinema... is anything but good and revolutionary" },
    Anchor { id: "adorno/letter_stigma", author: "adorno", verb: "argues", subject: "letter/high_low_art_stigma", predicate: "claim", object: "Both bear the stigma of capitalism, both contain elements of change" },
    Anchor { id: "adorno/letter_fear", author: "adorno", verb: "asserts", subject: "letter/abolition_of_fear", predicate: "claim", object: "The goal of the revolution is the abolition of fear" },
    // Adorno & Horkheimer, "The Culture Industry: Enlightenment as Mass Deception" (1944).
    Anchor { id: "adorno/ci_monopoly", author: "adorno", verb: "argues", subject: "culture_industry/monopoly_uniformity", predicate: "claim", object: "Under monopoly all mass culture is identical, and the lines of its artificial framework begin to show through" },
    Anchor { id: "adorno/ci_amusement", author: "adorno", verb: "argues", subject: "culture_industry/amusement_as_labor", predicate: "claim", object: "Amusement under late capitalism is the prolongation of work" },
    Anchor { id: "adorno/ci_technology", author: "adorno", verb: "argues", subject: "culture_industry/technological_rationale", predicate: "claim", object: "A technological rationale is the rationale of domination itself" },
    Anchor { id: "adorno/ci_movies_business", author: "adorno", verb: "asserts", subject: "culture_industry/movies_radio_business", predicate: "claim", object: "Movies and radio need no longer pretend to be art. The truth that they are just business is made into an ideology" },
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
    round: u32, // 0 for the seeded anchors, else the real round number
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
    round: u32,
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
        "Four Olympians (Athena, Artemis, Apollo, Dionysus) are analyzing two texts in real \
tension with each other, in a real, growing, checkable DMML commit log: Walter Benjamin's \"The \
Work of Art in the Age of Mechanical Reproduction\" (respondent=benjamin) and Theodor Adorno's real, \
documented direct critique of that exact essay -- his 18 March 1936 letter to Benjamin, plus \
Adorno & Horkheimer's \"The Culture Industry\" (respondent=adorno). Benjamin is broadly optimistic \
that mechanical reproduction can be politically emancipatory; Adorno is broadly skeptical, arguing \
mass culture standardizes and dominates rather than liberates. You have full autonomy to interpret \
EITHER author, counter-interpret one against the other, side with one against the other, or find a \
synthesis neither author states -- whatever a real reader with your temperament would actually do. \
Other entries in the log are prior turns from you or the other three Olympians.\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona -- agreeing with, \
extending, disputing, or connecting something SPECIFIC already in the log (Benjamin's claims, \
Adorno's claims, or a prior Olympian's turn), in a way another Olympian with your temperament \
actually would. Not a summary of either author, not a restatement. If you don't have a real move to \
make, say so honestly in `object` rather than padding. Call submit_dmml_turn with your answer. \
`consumes` must copy at least one real (cid, subject, predicate) from the log above exactly -- \
never invent one.",
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
    round: u32,
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
        round,
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

    let benjamin_author = api.author_create().await?;
    let adorno_author = api.author_create().await?;
    let mut source_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    source_authors.insert("benjamin", benjamin_author);
    source_authors.insert("adorno", adorno_author);
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        benjamin_author,
        "pantheon-olympians".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} real anchor claims (benjamin + adorno) --", ANCHORS.len());
    for a in ANCHORS {
        let author = source_authors[a.author];
        let mut rec = append(&substrate, &author, 0, a.verb, a.subject, a.predicate, a.object, &[]).await?;
        rec.respondent = a.author.to_string();
        println!("  [{}] {} -> {} \"{}\"", a.id, rec.cid, rec.subject, rec.object);
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
                        round as u32,
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
            round: t.round,
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
