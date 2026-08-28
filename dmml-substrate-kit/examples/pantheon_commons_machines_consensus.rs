//! Consensus stage for phase 7, "does the negative gate exit power."
//! Takes `pantheon_commons_machines.rs`'s transcript (the sovereignty
//! consensus tested against Deleuze/Guattari and Foucault) and attempts
//! ratification. Same accept/amend mechanism as every other
//! `pantheon_*consensus.rs` file. This round's transcript is unusual:
//! nearly every salvage proposed across five rounds was itself ruptured
//! by the next turn re-applying the same logic, converging on the
//! finding that no operation, rule, or meta-rule survives naming --
//! the consensus this stage has to draft is therefore explicitly about
//! a debate that resisted closure, not merely one that reached a tidy
//! answer.

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
const TRANSCRIPT_PATH: &str = "../dev-journal/artifacts/2026-08-28-pantheon-commons-machines-transcript.txt";

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
among four Olympians -- Athena, Artemis, Apollo, Dionysus -- extending a real sovereignty consensus \
document (itself the product of a long chain, ending in a NEGATIVE non-adjudication rule: anyone who \
claims erasure may convene the commune's process, but no one may adjudicate the claim in advance, and a \
legitimate commune must be able to dissolve, cease being the one who permits) using two new real sources: \
Deleuze and Guattari (desire as productive machinic connection, the rhizome as decentered and rootless, the \
war machine as exterior to the State apparatus) and Michel Foucault (power as relational and produced at \
every point, never a possession an institution can renounce; discipline's most effective form trains \
subjects to self-monitor rather than coercing them externally). This debate had an unusual shape: nearly \
every proposed salvage of a sovereignty_doc item -- an external attestor, a break-list of ruptures, a ban \
on specifying the outside, a plural-codes registry, an enforced-ignorance record -- was itself ruptured by \
the NEXT turn, which showed the salvage had quietly reinstalled an examiner or specified an outside it \
could not actually hold. The debate converged, through this repeated self-application, on several real \
findings: (1) item3's dissolution clause is honest only as naming PERPETUAL EXPOSURE to being found still \
permitting, never an achieved exit -- Foucault's point that power cannot be renounced because it was never \
a possession; (2) the pantheon's own checkpointed procedure (four fixed Olympians, numbered items, a \
ratification vote) was, all four Olympians concluded in their own reflections, an arborescent trunk \
throughout, NOT a rhizome -- the one genuinely rhizomatic element was material arriving from outside the \
document (Foucault and D&G themselves), which a tree can only host briefly before capturing it; (3) item0 \
('every repair installs an examiner') only worked as an unnamed, applied-as-against test -- the moment \
Apollo named and crowned it as the debate's confirmed content, naming itself became the next capture, so \
item0 survives only uncited, never as a confirmed rule; (4) even the debate's honest self-corrections \
(Athena's 'external attestor,' Dionysus's pre-confessed capture) were shown to be one more version of the \
same trap (appointment by description, inoculation-in-advance); (5) all four Olympians converged \
independently, in their reflection turns, on naming honesty itself as 'a rhythm of being corrected' rather \
than any achieved design or rule.\n\n\
=== TRANSCRIPT ===\n{transcript}\n=== END TRANSCRIPT ===\n\n\
You go first. Propose a candidate synthesis: an ordered sequence of 6-10 short, plain-language statements \
that honestly reflects what this debate actually established. The transcript does NOT support either 'the \
sovereignty consensus's negative gate is vindicated as rhizomatic' (refuted repeatedly) or 'nothing can be \
said, the debate collapsed into pure negation' (all four Olympians converged on real, specific findings, \
including about their own procedure) -- it supports the narrower, self-implicating findings above: \
dissolution as perpetual exposure not exit, the pantheon's own procedure named honestly as arborescent, \
item0 as unciteable-once-confirmed, and honesty as a rhythm of correction rather than a settled design. Your \
sequence should say this precisely, and should be honest that this synthesis itself is subject to the same \
capture the debate found in every prior synthesis -- it is fully legitimate for your sequence to say so \
explicitly. Call submit_vote with vote=\"propose\" and your sequence in `revised_sequence`."
        ),
        Some(seq) => format!(
            "Below is the same real transcript, and below that, the CURRENT candidate synthesis (the \
group is trying to reach unanimous agreement on a sequence that honestly reflects what this debate actually \
established, including that the pantheon's own procedure was arborescent, not rhizomatic, and that honesty \
here is a rhythm of correction, not an achieved design).\n\n\
=== TRANSCRIPT ===\n{transcript}\n=== END TRANSCRIPT ===\n\n\
=== CURRENT CANDIDATE SYNTHESIS ===\n{}\n=== END CANDIDATE ===\n\n\
Respond in your own voice as this persona. If this candidate honestly reflects what the debate actually \
established -- without smuggling back a tidy resolution the transcript's own recursive pattern dismantled, \
and without claiming more certainty about the pantheon's own procedure than the reflections earned -- call \
submit_vote with vote=\"accept\" and say briefly why. If something is wrong, missing, flattens the debate's \
real self-critique into false harmony, or reintroduces a naming/capture the transcript ruled out, call \
submit_vote with vote=\"amend\" and provide the COMPLETE replacement sequence in `revised_sequence` (not a \
patch -- the whole thing), plus your reason. Do not amend for the sake of amending -- if the current text is \
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
    // Same flash-model failure mode documented in
    // pantheon_commons_cyberpunk_consensus.rs and
    // pantheon_commons_sovereignty_consensus.rs: the initial-draft call
    // (current=None) can omit the required "vote" field entirely on an
    // otherwise well-formed response. "propose" is the only legal value
    // there, so default to it rather than error.
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
        created_at: "2026-08-28T00:00:00Z".to_string(),
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
        created_at: "2026-08-28T00:00:00Z".to_string(),
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
            uri: format!("iroh://pantheon-commons-machines-consensus/{}", p.cid),
            cid: p.cid.clone(),
        },
        subject: format!("consensus_v{}_item0", p.round),
        predicate: "statement".to_string(),
        object: None,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("== pantheon commons machines consensus: closing or holding the machines extension ==\n");

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
        "pantheon-commons-machines-consensus".to_string(),
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
    std::fs::write("pantheon_commons_machines_consensus.json", &json)?;
    println!("\nwrote pantheon_commons_machines_consensus.json");

    Ok(())
}
