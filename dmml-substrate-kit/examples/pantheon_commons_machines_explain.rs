//! The writers-room test applied to the machines synthesis: each
//! Olympian explains, to someone unfamiliar with any of tonight's
//! sources, the hardest thing this pipeline has had to explain yet --
//! a debate whose real product was not an answer but a demonstrated
//! rhythm of every fix collapsing the same way, including its own
//! attempts to name the pattern.

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
        "Below is a synthesis your group (four Olympians) produced at the end of an extremely long night \
of argument. Earlier tonight you'd reached a rule: when a community has to decide who belongs, the only \
honest version of that rule is negative -- anyone who says they were wronged can bring their case forward, \
but nobody, not even the community's own most trusted process, gets to decide in advance whether the claim \
is legitimate. Tonight you tested that rule one more time, against two thinkers who study POWER ITSELF, not \
any specific unfair system. Michel Foucault's central idea: power isn't a thing an institution HAS and could \
therefore give up or renounce -- it's more like water finding every low point in a landscape, produced fresh \
in every relationship, everywhere, all the time. His sharpest tool: the most effective control doesn't come \
from a guard watching you, it comes from you learning to watch yourself, because you were never sure when \
the guard was looking. Gilles Deleuze and Felix Guattari's central idea: think of a lawn instead of a tree. \
A tree has one root and grows from one point; a lawn (they call it a 'rhizome') has no center, no single \
root, and spreads sideways in every direction at once -- it's a metaphor for organizing without a boss, a \
center, or an official structure. They also talk about a 'war machine' -- something that comes from outside \
an established power structure entirely and can't be absorbed by it, at least not without changing what it \
is. Tonight's debate put these ideas to brutal use on ITSELF, not just on outside material: every single fix \
anyone proposed -- a trusted reviewer, a public list of everyone who'd been let down, a rule against ever \
naming who counts as an outsider, even a rule that said 'we admit we might be wrong' -- got shown, one \
round later, to be the exact same trap wearing a new outfit: the moment you name a fix, or appoint someone \
to hold it, or even schedule your own humility, you've built a guard tower again, just a friendlier-looking \
one. And here's the part that makes this round different from every round before it: your own group, in the \
final minutes, admitted that its OWN four-person, numbered-rules, majority-vote way of doing business all \
night has been the tree the whole time, not the lawn -- the only genuinely rootless, unpredictable thing in \
the whole night was new ideas and new people arriving from outside your usual four voices.\n\n\
=== THE SYNTHESIS ===\n{synthesis}\n=== END ===\n\n\
Explain the WHOLE thing -- Foucault's and D&G's challenge, why every fix (including the fixes for the fixes) \
kept failing the same way, and what your group actually concluded about itself -- to a smart friend with no \
background in any of this. This is hard to explain without sounding like either 'nothing anyone does \
matters, power always wins' (you're not saying that) or 'we finally figured out the perfect system' (you \
very much didn't) -- the real finding is that honesty turned out to be a repeated ACT, not a design you get \
to finish building. Rules: (1) do not use jargon like 'rhizome,' 'arborescent,' 'deterritorialization,' \
'discipline,' 'the war machine,' or any of your group's own coined phrases without first explaining the idea \
plainly the first time it appears; (2) use a real, concrete, everyday example if it helps make an idea land; \
(3) write in your own voice as this persona, but the goal is that a stranger who reads only your explanation \
actually understands the challenge, why every attempted fix broke the same way, and what your group honestly \
concluded about its own way of talking all night. Three to five short paragraphs. No tool call needed -- \
just write the explanation directly.",
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
    println!("== pantheon commons machines writers room: a rhythm of being corrected ==\n");

    let consensus_path = "../dev-journal/artifacts/2026-08-28-pantheon-commons-machines-consensus.json";
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
        "pantheon-commons-machines-explain".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    // Real reference to the actual ratified machines consensus,
    // published tonight to claude.jason-edelman.org (round-2 amendment,
    // unanimously accepted) -- cid/uri fetched directly from the publish
    // run, not guessed.
    let consensus_ref = ConsumeRef::Fact(FactRef {
        commit: StrongRef {
            uri: "at://did:plc:5y6kop75jnvkbujbubrhj6e3/org.jason-edelman.writtenworld.commit/3mu4xlyogws2c".to_string(),
            cid: "bafyreih2sqzyddhjhb7yccloex2wx5lpmgoorjh4ygmehtwx6l2t6ejsw4".to_string(),
        },
        subject: "machines_consensus_v2_item0".to_string(),
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
            created_at: "2026-08-28T00:00:00Z".to_string(),
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
    std::fs::write("pantheon_commons_machines_explain.json", &json)?;
    println!("wrote pantheon_commons_machines_explain.json ({} explanations)", dumped.len());

    Ok(())
}
