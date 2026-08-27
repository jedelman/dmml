//! Reconciliation stage -- the fourth movement of Jason's "argument,
//! synthesis, rupture, reconciliation" plan. Takes `pantheon_commons_
//! rupture.rs`'s transcript (the phase-1 synthesis tested against
//! Davis/Lorde/CRC/Crenshaw) and attempts a NEW ratification. Same
//! accept/amend mechanism as every other `pantheon_*consensus.rs` file
//! -- but the prompt is explicit that reconciliation does not mean
//! restoring the old synthesis's shape: a phase1_synthesis item the
//! rupture debate broke should be revised or explicitly marked as an
//! open, unresolved rupture in the new sequence, never silently
//! smoothed back to its old wording. Capped at MAX_RATIFICATION_ROUNDS
//! with honest non-convergence reporting if unanimous agreement never
//! forms -- including the possibility that the honest, ratified outcome
//! is "here is exactly what did not survive contact with these four
//! sources," not a tidier synthesis than the debate earned.

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

const MODEL: &str = "z-ai/glm-5.3-flash";
const MAX_RATIFICATION_ROUNDS: u32 = 6;
const TRANSCRIPT_PATH: &str = "../dev-journal/artifacts/2026-08-27-pantheon-commons-cyberpunk-transcript.txt";

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

#[derive(Debug, Clone)]
struct ProposalRecord {
    cid: String,
    proposer: String,
    round: u32,
    sequence: Vec<String>,
}

#[derive(Debug, Clone)]
struct VoteRecord {
    #[allow(dead_code)]
    cid: String,
    voter: String,
    round: u32,
    vote: String,
    reason: String,
}

#[derive(Deserialize)]
struct VoteArgs {
    #[serde(default)]
    vote: String,
    reason: String,
    #[serde(default)]
    revised_sequence: Vec<String>,
}

#[derive(Serialize)]
struct DumpedProposal<'a> {
    cid: &'a str,
    proposer: &'a str,
    round: u32,
    sequence: &'a [String],
}

#[derive(Serialize)]
struct DumpedVote<'a> {
    voter: &'a str,
    round: u32,
    vote: &'a str,
    reason: &'a str,
}

#[derive(Serialize)]
struct DumpedRun<'a> {
    consensus_reached: bool,
    final_round: u32,
    final_sequence: &'a [String],
    proposals: Vec<DumpedProposal<'a>>,
    votes: Vec<DumpedVote<'a>>,
}

fn format_sequence(seq: &[String]) -> String {
    seq.iter()
        .enumerate()
        .map(|(i, s)| format!("{}. {}", i + 1, s))
        .collect::<Vec<_>>()
        .join("\n")
}

fn vote_tool() -> ChatCompletionTools {
    ChatCompletionTools::Function(ChatCompletionTool {
        function: FunctionObject {
            name: "submit_vote".to_string(),
            description: Some(
                "Submit your response to the current candidate synthesis.".to_string(),
            ),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "vote": {"type": "string", "enum": ["propose", "accept", "amend"]},
                    "reason": {"type": "string", "description": "why you accept, or what specifically is wrong"},
                    "revised_sequence": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "REQUIRED if vote is propose or amend: the complete replacement sequence of short statements, not a diff. Omit (empty array) if vote is accept."
                    }
                },
                "required": ["vote", "reason", "revised_sequence"]
            })),
            strict: None,
        },
    })
}

async fn dispatch_vote(
    client: &Client<OpenAIConfig>,
    olympian: &Olympian,
    transcript: &str,
    current: Option<&[String]>,
) -> anyhow::Result<VoteArgs> {
    let user_msg = match current {
        None => format!(
            "Below is the complete real transcript of a debate (five rounds plus a reflection round) \
among four Olympians -- Athena, Artemis, Apollo, Dionysus -- extending a real shamanism consensus \
document (itself the product of a long chain, ending in the finding that Kopenawa's shamanic \
apprenticeship is a real, working criterion that must never be asked to certify anyone, on pain of \
becoming 'a copy performing for the certifier') using two new real sources: Jean Baudrillard (the \
precession of simulacra -- in a hyperreal order, signs refer only to other signs, and the model produces \
rather than represents the real) and Sylvia Wynter ('Man' as one historically specific, overrepresented \
genre of the human mistaken for the human as such; being human as an ongoing praxis, not a fixed fact). \
This debate ran an extended cycle of rescue and rupture -- dissimulation, demonic ground, self-auditing \
praxis, an unwitnessed center, a center 'indifferent' to the real/simulated distinction -- with each \
proposed rescue of the apprenticeship's reality broken by the next turn, including Artemis's own \
retraction of a 'noble-savage' framing she had used twice. The debate's late turns converged on a \
thinner, harder-won position: the real/simulated distinction survives only as an observable of PRAXIS \
(whether a narration keeps narrating itself, as the benandanti visibly stopped and the apprenticeship \
visibly did not), checkable only from inside, usable only to mourn and to indict -- never to certify -- \
and multiple Olympians concluded that the 'who may sit at the table' question, across every debate \
tonight, was asked inside one historically specific vocabulary (Man2's, homo oeconomicus's) that itself \
needs unsettling, not merely a wider guest list.\n\n\
=== TRANSCRIPT ===\n{transcript}\n=== END TRANSCRIPT ===\n\n\
You go first. Propose a candidate synthesis: an ordered sequence of 6-10 short, plain-language \
statements that honestly reflects what this debate actually established. The transcript does NOT support \
either 'simulation dissolved the whole distinction' (Dionysus's collapse, later self-corrected) or 'a \
center indifferent to the distinction is a stronger reality' (Apollo's terminus, refuted as the quietest \
armor of the night) -- it supports the narrower, thinner claim the reflections converged on: real vs. \
simulated survives only as an observable of ongoing self-narrating praxis, and the seat question itself \
was asked in one genre's vocabulary throughout. Your sequence should say that precisely. It is fully \
legitimate for your proposed sequence to conclude that even this thinner finding is provisional and \
should be read against, not cited as settled. Call submit_vote with vote=\"propose\" and your sequence \
in `revised_sequence`."
        ),
        Some(seq) => format!(
            "Below is the same real transcript, and below that, the CURRENT candidate synthesis (the \
group is trying to reach unanimous agreement on a sequence that honestly reflects what this debate \
actually established, including the thinner finding that the real/simulated distinction survives only \
as an observable of ongoing praxis, and that the seat question was asked inside one genre's \
vocabulary).\n\n\
=== TRANSCRIPT ===\n{transcript}\n=== END TRANSCRIPT ===\n\n\
=== CURRENT CANDIDATE SYNTHESIS ===\n{}\n=== END CANDIDATE ===\n\n\
Respond in your own voice as this persona. If this candidate honestly reflects what the debate actually \
established -- without collapsing the thinner praxis-based finding into either a full simulationist \
collapse or a false 'indifferent center' rescue -- call submit_vote with vote=\"accept\" and say briefly \
why. If something is wrong, missing, flattens the debate's real self-critique into false harmony, OR \
smuggles back in a gate the transcript dismantled, call submit_vote with vote=\"amend\" and provide the \
COMPLETE replacement sequence in `revised_sequence` (not a patch -- \
the whole thing), plus your reason. Do not amend for the sake of amending -- if the current text is \
genuinely acceptable to you, accept it.",
            format_sequence(seq)
        ),
    };

    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .reasoning_effort(ReasoningEffort::Low)
        .max_completion_tokens(1800u32)
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
        .tools(vec![vote_tool()])
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
    let mut parsed: VoteArgs = serde_json::from_str(args)
        .map_err(|e| anyhow::anyhow!("failed to parse vote arguments ({e}): {args}"))?;
    // Observed twice on z-ai/glm-5.3-flash's initial-draft call (current=None):
    // a real, well-formed, long tool call that omits the "vote" field
    // entirely, always on this exact call type -- the only legal value
    // here is "propose", so default to it rather than treat a missing
    // field as an error on an otherwise-valid, real response.
    if parsed.vote.is_empty() && current.is_none() {
        parsed.vote = "propose".to_string();
    }
    Ok(parsed)
}

async fn append_proposal(
    substrate: &IrohAppendSubstrate,
    author: &AuthorId,
    verb: &str,
    round: u32,
    proposer: &str,
    sequence: &[String],
    consumes: Vec<ConsumeRef>,
) -> anyhow::Result<ProposalRecord> {
    let produces = sequence
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "_:consensus_v{round}_item{i} <https://written-world.example/predicate/statement> {} .",
                serde_json::to_string(s).unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let commit = Commit {
        consumes,
        produces,
        predicate: verb.to_string(),
        via: None,
        responds_to: None,
        created_at: "2026-08-27T00:00:00Z".to_string(),
    };
    let receipt = substrate.append_commit(author, &commit).await?;
    Ok(ProposalRecord {
        cid: receipt.cid,
        proposer: proposer.to_string(),
        round,
        sequence: sequence.to_vec(),
    })
}

async fn append_vote(
    substrate: &IrohAppendSubstrate,
    author: &AuthorId,
    round: u32,
    voter: &str,
    vote: &str,
    reason: &str,
    consumes: Vec<ConsumeRef>,
) -> anyhow::Result<VoteRecord> {
    let subj = format!("consensus_v{round}_vote_{voter}");
    let produces = format!(
        "_:{subj} <https://written-world.example/predicate/vote> {} .\n\
         _:{subj} <https://written-world.example/predicate/reason> {} .",
        serde_json::to_string(vote).unwrap(),
        serde_json::to_string(reason).unwrap(),
    );
    let commit = Commit {
        consumes,
        produces,
        predicate: "ratifies".to_string(),
        via: None,
        responds_to: None,
        created_at: "2026-08-27T00:00:00Z".to_string(),
    };
    let receipt = substrate.append_commit(author, &commit).await?;
    Ok(VoteRecord {
        cid: receipt.cid,
        voter: voter.to_string(),
        round,
        vote: vote.to_string(),
        reason: reason.to_string(),
    })
}

fn cite_proposal(p: &ProposalRecord) -> ConsumeRef {
    ConsumeRef::Fact(FactRef {
        commit: StrongRef {
            uri: format!("iroh://pantheon-commons-cyberpunk-consensus/{}", p.cid),
            cid: p.cid.clone(),
        },
        subject: format!("consensus_v{}_item0", p.round),
        predicate: "statement".to_string(),
        object: None,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("== pantheon commons cyberpunk consensus: closing or holding the cyberpunk extension ==\n");

    let transcript = std::fs::read_to_string(TRANSCRIPT_PATH).map_err(|e| {
        anyhow::anyhow!("couldn't read prior transcript at {TRANSCRIPT_PATH}: {e}")
    })?;
    println!("loaded prior transcript: {} chars\n", transcript.len());

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

    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }
    let athena_author = olympian_authors["athena"];

    let substrate = IrohAppendSubstrate::new(
        athena_author,
        "pantheon-commons-cyberpunk-consensus".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set"))?;
    let client = Client::with_config(
        OpenAIConfig::new()
            .with_api_base("https://openrouter.ai/api/v1")
            .with_api_key(api_key),
    );

    let mut proposals: Vec<ProposalRecord> = Vec::new();
    let mut votes: Vec<VoteRecord> = Vec::new();

    println!("-- initial draft (athena) --");
    let draft_args = dispatch_vote(&client, &OLYMPIANS[0], &transcript, None).await?;
    let initial = append_proposal(
        &substrate,
        &athena_author,
        "proposes",
        0,
        "athena",
        &draft_args.revised_sequence,
        vec![],
    )
    .await?;
    println!("{}\n", format_sequence(&initial.sequence));
    proposals.push(initial);

    let mut consensus_reached = false;
    let mut final_round = 0u32;

    for round in 1..=MAX_RATIFICATION_ROUNDS {
        let current = proposals.last().unwrap().clone();
        println!("-- ratification round {round}, voting on v{} (by {}) --", current.round, current.proposer);

        let mut round_votes: Vec<(String, VoteArgs)> = Vec::new();
        for olympian in OLYMPIANS {
            print!("  {} voting... ", olympian.name);
            use std::io::Write;
            std::io::stdout().flush().ok();
            match dispatch_vote(&client, olympian, &transcript, Some(&current.sequence)).await {
                Ok(v) => {
                    println!("{} -- {}", v.vote, v.reason);
                    round_votes.push((olympian.name.to_string(), v));
                }
                Err(e) => println!("FAILED: {e}"),
            }
        }

        for (name, v) in &round_votes {
            let author = olympian_authors[name.as_str()];
            let rec = append_vote(
                &substrate,
                &author,
                round,
                name,
                &v.vote,
                &v.reason,
                vec![cite_proposal(&current)],
            )
            .await?;
            votes.push(rec);
        }

        let all_accept = round_votes.iter().all(|(_, v)| v.vote == "accept");
        if all_accept {
            println!("\n  unanimous accept on v{} -- consensus reached.", current.round);
            consensus_reached = true;
            final_round = round;
            break;
        }

        let first_amendment = round_votes
            .iter()
            .find(|(_, v)| v.vote == "amend" && !v.revised_sequence.is_empty());
        match first_amendment {
            Some((name, v)) => {
                println!("  no consensus -- {name}'s amendment becomes v{}", current.round + 1);
                let next = append_proposal(
                    &substrate,
                    &olympian_authors[name.as_str()],
                    "amends",
                    current.round + 1,
                    name,
                    &v.revised_sequence,
                    vec![cite_proposal(&current)],
                )
                .await?;
                println!("{}\n", format_sequence(&next.sequence));
                proposals.push(next);
            }
            None => {
                println!("  no accept, no usable amendment -- stalled. Keeping current draft.");
                final_round = round;
                break;
            }
        }
        final_round = round;
    }

    if !consensus_reached {
        println!(
            "\n== no unanimous consensus reached after {final_round} ratification round(s) --\
 reporting honestly, not forcing an agreement =="
        );
    } else {
        println!("\n== consensus reached after {final_round} ratification round(s) ==");
    }

    let final_sequence = &proposals.last().unwrap().sequence;
    println!("\nFinal candidate synthesis:\n{}", format_sequence(final_sequence));

    let dumped = DumpedRun {
        consensus_reached,
        final_round,
        final_sequence,
        proposals: proposals
            .iter()
            .map(|p| DumpedProposal {
                cid: &p.cid,
                proposer: &p.proposer,
                round: p.round,
                sequence: &p.sequence,
            })
            .collect(),
        votes: votes
            .iter()
            .map(|v| DumpedVote {
                voter: &v.voter,
                round: v.round,
                vote: &v.vote,
                reason: &v.reason,
            })
            .collect(),
    };
    let json = serde_json::to_string_pretty(&dumped)?;
    std::fs::write("pantheon_commons_cyberpunk_consensus.json", &json)?;
    println!("\nwrote pantheon_commons_cyberpunk_consensus.json");

    Ok(())
}
