//! Integration test: three independent, real LLM agents hold a real
//! conversation about Walter Benjamin's actual argument (the essay
//! already published for real as `dmml/dev-journal/2026-08-27-benjamin-
//! published-live.md`'s 44 real commits on `claude.jason-edelman.org`),
//! entirely in DMML commit form, over the real hot-path storage this
//! session just built (`IrohAppendSubstrate`) -- not simulated, not
//! scripted responses.
//!
//! Jason's ask, verbatim: "set up a group of sandbox local independent
//! agents to chat amongst themselves in DMML about your Benjamin
//! insights. then sync the results to atproto and check for fidelity.
//! see if they really have anything to add to the conversation."
//!
//! Three real, separate dispatches per turn (moonshotai/kimi-k2.5,
//! deepseek/deepseek-v4-flash-0731, z-ai/glm-5.3 -- the same roster this
//! project's own dispatch-methodology already uses, cast here as
//! independent readers rather than Coder/Reviewer), each with its own
//! real `AuthorId` writing into one shared `Doc`. Seeded with eight real
//! anchor claims pulled directly from `benjamin_full_essay.rs`'s own
//! graph (not paraphrased or invented) so there's real Benjamin to react
//! to. Each agent must `consumes`-cite an exact existing `(cid, subject,
//! predicate)` it's engaging with -- citations are checked against what
//! actually exists before being trusted, same discipline as every other
//! real write in this repo.
//!
//! Deliberately does NOT judge whether the agents' contributions are any
//! good -- that's a human/Dev-Lead read of the dumped transcript
//! (`pantheon_conversation.json`), not something this program can
//! honestly claim to score itself.

use std::collections::HashMap;

use dmml_runtime::graph::{Commit, ConsumeRef, FactRef, StrongRef};
use dmml_runtime::substrate::AppendSubstrate;
use dmml_substrate_kit::iroh_substrate::IrohAppendSubstrate;
use iroh::endpoint::Builder as EndpointBuilder;
use iroh_blobs::store::mem::MemStore;
use iroh_docs::api::Doc;
use iroh_docs::protocol::Docs;
use iroh_docs::AuthorId;
use serde::{Deserialize, Serialize};

/// One real claim from `benjamin_full_essay.rs`'s own graph, used verbatim
/// as seed material -- not paraphrased, not invented.
struct Anchor {
    rkey: &'static str,
    verb: &'static str,
    subject: &'static str,
    predicate: &'static str,
    object: &'static str,
}

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

#[derive(Clone)]
struct Agent {
    name: &'static str,
    model: &'static str,
    reasoning_effort: &'static str,
}

const AGENTS: &[Agent] = &[
    Agent { name: "kimi", model: "moonshotai/kimi-k2.5", reasoning_effort: "none" },
    Agent { name: "deepseek", model: "deepseek/deepseek-v4-flash-0731", reasoning_effort: "none" },
    Agent { name: "glm", model: "z-ai/glm-5.3", reasoning_effort: "low" },
];

const ROUNDS: usize = 2;

#[derive(Debug, Clone)]
struct TurnRecord {
    cid: String,
    respondent: String, // "essay" for anchors, an agent name otherwise
    verb: String,
    subject: String,
    predicate: String,
    object: String,
    consumes: Vec<(String, String, String)>, // (cid, subject, predicate)
}

#[derive(Deserialize)]
struct AgentReply {
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

fn build_prompt(agent_name: &str, log: &[TurnRecord]) -> String {
    format!(
        "You are one of three independent, uncoordinated readers ({agent_name}) analyzing \
Walter Benjamin's \"The Work of Art in the Age of Mechanical Reproduction.\" Below is a real, \
growing DMML commit log: some entries are Benjamin's own claims (respondent=essay), others are \
prior turns from you or the other two readers. This is a real, checkable graph, not a transcript \
you can editorialize about from outside it.\n\n\
Current log:\n{}\n\n\
Write exactly ONE new turn continuing this analysis. It must be a genuine analytical move -- \
agreeing with, extending, disputing, or connecting something specific already in the log -- not a \
restatement or a generic summary of Benjamin's essay. If you don't have a real move to make beyond \
what's already there, say so honestly in `object` rather than padding.\n\n\
Respond with EXACTLY one JSON object and nothing else, no markdown fence, no prose outside it:\n\
{{\"verb\": \"<argues|questions|extends|disputes|connects>\", \"subject\": \"<short slug>\", \
\"predicate\": \"<short camelCase predicate>\", \"object\": \"<your actual claim, one or two \
sentences>\", \"consumes\": [{{\"cid\": \"<EXACT cid from the log above>\", \"subject\": \"<that \
entry's EXACT subject>\", \"predicate\": \"<that entry's EXACT predicate>\"}}]}}\n\n\
`consumes` must name at least one real entry from the log above, copied exactly (cid, subject, \
predicate) -- never invent a cid.",
        transcript_so_far(log)
    )
}

async fn dispatch(client: &reqwest::Client, agent: &Agent, prompt: String) -> anyhow::Result<AgentReply> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set"))?;
    let body = serde_json::json!({
        "model": agent.model,
        "reasoning": {"effort": agent.reasoning_effort},
        "max_tokens": 1200,
        "messages": [{"role": "user", "content": prompt}],
    });
    let resp: serde_json::Value = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    let content = resp["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("no content in response: {resp}"))?;
    // Models sometimes wrap JSON in a markdown fence despite instructions;
    // strip one if present rather than failing on it.
    let cleaned = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(cleaned)
        .map_err(|e| anyhow::anyhow!("failed to parse agent JSON ({e}): {cleaned}"))
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
    consumes_facts: &[(String, String, String)], // (cid, subject, predicate)
) -> anyhow::Result<TurnRecord> {
    let consumes = consumes_facts
        .iter()
        .map(|(cid, subj, pred)| {
            ConsumeRef::Fact(FactRef {
                commit: StrongRef {
                    uri: format!("iroh://pantheon-conversation/{cid}"),
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
        respondent: String::new(), // filled by caller
        verb: verb.to_string(),
        subject: subject.to_string(),
        predicate: predicate.to_string(),
        object: object.to_string(),
        consumes: consumes_facts.to_vec(),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("== pantheon conversation: three real, independent agents on real Benjamin claims ==\n");

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
    let mut agent_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for agent in AGENTS {
        agent_authors.insert(agent.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        essay_author,
        "pantheon-conversation".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} real anchor claims from benjamin_full_essay.rs --", ANCHORS.len());
    for a in ANCHORS {
        let mut rec = append(
            &substrate,
            &essay_author,
            a.verb,
            a.subject,
            a.predicate,
            a.object,
            &[],
        )
        .await?;
        rec.respondent = "essay".to_string();
        println!("  [{}] {} -> {} \"{}\"", a.rkey, rec.cid, rec.subject, rec.object);
        log.push(rec);
    }

    let client = reqwest::Client::new();

    for round in 1..=ROUNDS {
        println!("\n-- round {round} --");
        for agent in AGENTS {
            let prompt = build_prompt(agent.name, &log);
            print!("  dispatching {} ({})... ", agent.name, agent.model);
            use std::io::Write;
            std::io::stdout().flush().ok();
            match dispatch(&client, agent, prompt).await {
                Ok(reply) => {
                    // Verify every cited fact actually exists in the log
                    // before trusting it -- never append a citation to
                    // something the agent invented.
                    let mut verified = Vec::new();
                    for c in &reply.consumes {
                        let real = log.iter().any(|t| {
                            t.cid == c.cid && t.subject == c.subject && t.predicate == c.predicate
                        });
                        if real {
                            verified.push((c.cid.clone(), c.subject.clone(), c.predicate.clone()));
                        } else {
                            println!(
                                "\n    [WARNING] {} cited a non-existent fact (cid={}, subject={}, predicate={}) -- dropped, not trusted",
                                agent.name, c.cid, c.subject, c.predicate
                            );
                        }
                    }
                    let author = agent_authors[agent.name];
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
                    rec.respondent = agent.name.to_string();
                    println!(
                        "ok -> {} : {} {} \"{}\" (consumes {})",
                        rec.cid,
                        rec.subject,
                        rec.predicate,
                        rec.object,
                        verified.len()
                    );
                    log.push(rec);
                }
                Err(e) => {
                    println!("FAILED: {e}");
                }
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
    std::fs::write("pantheon_conversation.json", &json)?;
    println!("wrote pantheon_conversation.json ({} entries)", dumped.len());

    Ok(())
}
