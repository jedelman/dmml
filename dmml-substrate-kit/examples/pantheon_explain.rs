//! The "writers room" test, per Jason's challenge: "can any (or all) of
//! them explain the argument to someone unfamiliar with either texts or
//! the argument?" The whole debate accumulated real internal jargon --
//! "the check," "the nose," "prosthetic witness," "the morning-after
//! appetite test" -- coined mid-argument and never defined for an
//! outside reader. This is the test of whether there's real
//! understanding underneath that vocabulary or just performed erudition
//! that collapses once asked to explain itself in plain terms.
//!
//! Each of the four Olympians is given the ratified consensus synthesis
//! (`pantheon_consensus.rs`'s real output) and asked, in its own voice,
//! to explain the whole argument to someone who has never read Benjamin
//! or Adorno and has no philosophy background -- explicitly forbidden
//! from using its own debate's coined shorthand without first defining
//! it in plain terms. No tool-calling/citation discipline this time --
//! this is a writing task, not a claim needing verification, though the
//! result is still appended as a real DMML commit (verb `explains`)
//! citing the consensus proposal, for continuity with everything else
//! this evening.

use std::collections::HashMap;

use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs, ReasoningEffort,
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

const MODEL: &str = "z-ai/glm-5.3-flash";

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
another speaker; you go looking for what a proposal has overlooked or is \
too polite to say.",
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

#[derive(Deserialize)]
struct ConsensusRun {
    final_sequence: Vec<String>,
}

fn format_sequence(seq: &[String]) -> String {
    seq.iter()
        .enumerate()
        .map(|(i, s)| format!("{}. {}", i + 1, s))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn explain(
    client: &Client<OpenAIConfig>,
    olympian: &Olympian,
    synthesis: &str,
) -> anyhow::Result<String> {
    let user_msg = format!(
        "Below is a philosophical synthesis your group (four Olympians) reached after a long \
debate about Walter Benjamin's essay on mechanical reproduction and Theodor Adorno's real critique \
of it. It uses vocabulary your group coined mid-debate -- terms like \"the check,\" \"the nose,\" \
\"the morning-after,\" \"testimony migrating\" -- that mean nothing to someone who wasn't in the room.\n\n\
=== THE SYNTHESIS ===\n{synthesis}\n=== END ===\n\n\
Explain the WHOLE argument -- what real question it's actually about, what the core disagreement \
was, and what you all ultimately agreed on -- to a smart friend who has never read Benjamin or \
Adorno, doesn't know what \"aura\" means in this context, and has no philosophy background at all. \
Rules: (1) do not use any of your group's coined shorthand without first explaining it in ordinary \
words the first time it appears -- no \"the check\" or \"the nose\" used as if the reader already \
knows what that means; (2) use a real, concrete, everyday example if it helps (concerts, celebrities, \
photographs, whatever actually clarifies it -- not abstract restatement); (3) write in your own \
voice as this persona, but the goal is that a stranger who reads only your explanation, never the \
original debate, walks away actually understanding what was at stake and what was decided. Three to \
five short paragraphs. No tool call needed -- just write the explanation directly.",
    );

    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .reasoning_effort(ReasoningEffort::Low)
        .max_completion_tokens(1200u32)
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
        .build()?;

    let response = client.chat().create(request).await?;
    let content = response
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .ok_or_else(|| anyhow::anyhow!("no content in response"))?;
    Ok(content)
}

#[derive(Serialize)]
struct DumpedExplanation<'a> {
    cid: &'a str,
    explainer: &'a str,
    explanation: &'a str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("== pantheon writers room: explain the argument to an outsider ==\n");

    let consensus_path = "../dev-journal/artifacts/2026-08-27-pantheon-consensus.json";
    let raw = std::fs::read_to_string(consensus_path)
        .map_err(|e| anyhow::anyhow!("couldn't read {consensus_path}: {e}"))?;
    let consensus: ConsensusRun = serde_json::from_str(&raw)?;
    let synthesis_text = format_sequence(&consensus.final_sequence);
    println!("loaded ratified synthesis ({} statements)\n", consensus.final_sequence.len());

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

    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }
    let athena_author = olympian_authors["athena"];
    let substrate = IrohAppendSubstrate::new(
        athena_author,
        "pantheon-explain".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    // Real, standalone reference to the actual consensus proposal
    // published earlier tonight to claude.jason-edelman.org -- cited as
    // provenance, not re-derived. cid fetched directly via
    // com.atproto.repo.getRecord before writing this, not guessed --
    // this is a cross-substrate quotation (per ARCHITECTURE.md's own
    // "cross-DID references stay quotation, not verification" design),
    // not something this local doc's own resolve_fact can check.
    let consensus_ref = ConsumeRef::Fact(FactRef {
        commit: StrongRef {
            uri: "at://did:plc:5y6kop75jnvkbujbubrhj6e3/org.jason-edelman.writtenworld.commit/3mu3ar5pdqv25".to_string(),
            cid: "bafyreiav7qwxfrisuxqzlm5lvilmlf262dxp3wwiqblzwnma5s5ohqehpm".to_string(),
        },
        subject: "consensus_v0_item0".to_string(),
        predicate: "statement".to_string(),
        object: None,
    });

    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set"))?;
    let client = Client::with_config(
        OpenAIConfig::new()
            .with_api_base("https://openrouter.ai/api/v1")
            .with_api_key(api_key),
    );

    let mut explanations = Vec::new();
    for olympian in OLYMPIANS {
        println!("-- {} explaining --", olympian.name);
        let text = explain(&client, olympian, &synthesis_text).await?;
        println!("{text}\n");

        let commit = Commit {
            consumes: vec![consensus_ref.clone()],
            produces: format!(
                "_:explains_{} <https://written-world.example/predicate/plainExplanation> {} .",
                olympian.name,
                serde_json::to_string(&text).unwrap()
            ),
            predicate: "explains".to_string(),
            via: None,
            responds_to: None,
            created_at: "2026-08-27T00:00:00Z".to_string(),
        };
        let author = olympian_authors[olympian.name];
        let receipt = substrate.append_commit(&author, &commit).await?;
        explanations.push((receipt.cid, olympian.name.to_string(), text));
    }

    let dumped: Vec<DumpedExplanation> = explanations
        .iter()
        .map(|(cid, name, text)| DumpedExplanation {
            cid,
            explainer: name,
            explanation: text,
        })
        .collect();
    let json = serde_json::to_string_pretty(&dumped)?;
    std::fs::write("pantheon_explain.json", &json)?;
    println!("wrote pantheon_explain.json ({} explanations)", dumped.len());

    Ok(())
}
