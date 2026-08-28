//! The writers-room test applied to the sovereignty synthesis: each
//! Olympian explains, to someone unfamiliar with any of tonight's
//! sources, what an entire night's debate about "who belongs at the
//! table" turned into once real, literal life-and-death stakes and a
//! real, functioning institution were finally put in the room.

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
        "Below is a synthesis your group (four Olympians) produced at the end of a very long night of \
argument. Across many earlier rounds you had reached a hard-won, thin position: a real spiritual practice \
can be authentic, but the moment you turn 'is it authentic' into a test for who belongs at a decision-making \
table, the test corrupts into a performance for the judge -- so the honest rule was 'never certify, only \
mourn a loss and indict whoever caused it.' Tonight you tested that rule against real stakes and a real \
institution. Achille Mbembe, a philosopher, argues that political power, at bottom, is the power to decide \
who lives and who dies -- and shows this was literal, historical fact in real places: plantations, colonies, \
concentration camps, not just an idea. His challenge: if that's what power actually is, is 'never certify, \
only mourn' something anyone facing that kind of power can actually afford, or is it a comfort available \
only to people safely outside the danger? Abdullah Ocalan, a political theorist and organizer, built (and \
people are still building) a real alternative to the nation-state -- self-governing local communities \
(communes) that make their own decisions together, with an explicit rule that a community can't be truly \
free unless its women are free first. His challenge: does an actual, working, non-state community finally \
solve the 'who belongs' problem your group couldn't solve in the abstract, or does even a real, well- \
intentioned community still have to decide who counts as a member -- meaning the same trap reappears no \
matter how good the institution is? Your group spent five rounds trying to build a version of 'never \
certify' that could survive contact with these two challenges -- a trusted internal reviewer, a fixed \
schedule for renewing decisions, an archive that receives testimony afterward, a public record of everyone \
left out -- and every single version was shown, by the next speaker, to have quietly brought back some kind \
of judge in disguise. What finally survived was much smaller: anyone who says they were wronged gets to \
bring their case forward, but no one -- not even the community's own most trusted process -- gets to decide \
in advance whether that person's claim is legitimate before hearing it out. That openness will get abused \
sometimes, and your group concluded that this is the honest cost of the only version of the rule that \
doesn't quietly become a gate again -- not a flaw to be engineered away. And Ocalan's real institution did \
give you one genuine, new answer, just not the one you were looking for: it can't tell you WHO should get a \
say, but it does suggest an ORDER -- whoever's voice has historically been silenced or erased first is the \
first person whose situation the community has to address. An order of attention is not the same thing as a \
test people have to pass.\n\n\
=== THE SYNTHESIS ===\n{synthesis}\n=== END ===\n\n\
Explain the WHOLE thing -- Mbembe's and Ocalan's challenges, why every attempted fix quietly rebuilt the \
same judge, and what small, honest thing survived anyway -- to a smart friend with no background in any of \
this. This is hard to explain without sounding like either 'there's no hope, power always wins' (you're not \
saying that) or 'we finally solved it' (you didn't) -- the actual finding is narrower and more useful than \
either. Rules: (1) do not use jargon like 'necropolitics,' 'sovereignty,' 'confederalism,' 'adjudication,' \
or any of your group's own coined phrases without first explaining the idea in ordinary words; (2) use a \
real, concrete, everyday example if it helps make the idea land; (3) write in your own voice as this \
persona, but the goal is that a stranger who reads only your explanation actually understands the two \
challenges, why every fix broke, and what thin, honest practice was still standing at the end. Three to \
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
    println!("== pantheon commons sovereignty writers room: the negative gate ==\n");

    let consensus_path = "../dev-journal/artifacts/2026-08-28-pantheon-commons-sovereignty-consensus.json";
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
        "pantheon-commons-sovereignty-explain".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    // Real reference to the actual ratified sovereignty consensus,
    // published tonight to claude.jason-edelman.org (round-2 amendment,
    // unanimously accepted) -- cid/uri fetched directly from the publish
    // run, not guessed.
    let consensus_ref = ConsumeRef::Fact(FactRef {
        commit: StrongRef {
            uri: "at://did:plc:5y6kop75jnvkbujbubrhj6e3/org.jason-edelman.writtenworld.commit/3mu4vjowate27".to_string(),
            cid: "bafyreicjlvburz6pk2klr5vjgps3yftpg4agv6yf3l7docop42xpwahuza".to_string(),
        },
        subject: "sovereignty_consensus_v2_item0".to_string(),
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
    std::fs::write("pantheon_commons_sovereignty_explain.json", &json)?;
    println!("wrote pantheon_commons_sovereignty_explain.json ({} explanations)", dumped.len());

    Ok(())
}
