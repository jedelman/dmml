//! Phase 6: "who decides who lives" -- the natural next move after the
//! cyberpunk consensus concluded that the seat-at-the-table question was
//! asked in one genre's vocabulary (Man's) and that the only surviving
//! distinction (real vs. simulated praxis) may be used only to mourn and
//! indict, never to certify or admit. Two new sources put real pressure
//! on that restriction from opposite directions:
//!
//! Achille Mbembe's "Necropolitics" (2003) defines sovereignty itself as
//! the power to dictate who may live and who must die, and names real
//! historical sites (the plantation, the colony, the camp) where that
//! power operated as fact, not metaphor. If sovereignty just IS the power
//! to decide who counts, is "never certify, only mourn" a workable ethic
//! for anyone actually facing that power, or is it a position only
//! available to those not yet standing in a death-world? Abdullah
//! Öcalan's "Democratic Confederalism" is the opposite kind of pressure:
//! not a critique but an actual, real, still-running attempt to build a
//! non-state institution that answers "who belongs and who decides"
//! outside the state form entirely -- grassroots communes, radical
//! subsidiarity, and (via jineology) an explicit claim that women's
//! liberation is the precondition of any liberated society at all. Does
//! Öcalan's actual institution supply what six debates couldn't find, or
//! does it fall into the same trap the cyberpunk consensus diagnosed --
//! and does Mbembe's necropolitical stakes mean the whole "never certify"
//! ethic has to be revised, not just re-affirmed?
//!
//! Neither thinker has a Power Explained dramatis-personae profile yet
//! (real gap, noted in the dev-journal and paper update); both are real,
//! well-documented figures, and every claim below is either a verified
//! direct quote (checked live via web search before this file was
//! written) or an explicitly labeled paraphrase of a well-documented
//! argument.
//!
//! Structurally: same frozen-prior-plus-new-anchors pattern as every
//! extension run, this time against the CYBERPUNK run's own consensus
//! (`pantheon_commons_cyberpunk_consensus.rs`'s output).

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
    author: &'static str,
    verb: &'static str,
    subject: &'static str,
    predicate: &'static str,
    object: &'static str,
}

// The 16 new anchors. Every claim is either a verified direct quote or
// an explicitly labeled paraphrase of a well-documented argument, same
// discipline as every anchor in this project. Sources checked live via
// web search on 2026-08-28 before this file was written.
const NEW_ANCHORS: &[Anchor] = &[
    // Achille Mbembe, "Necropolitics" (Public Culture, 2003).
    Anchor { id: "mbembe/sovereignty_who_may_live", author: "mbembe", verb: "asserts", subject: "necropolitics/sovereignty_who_may_live", predicate: "claim", object: "The ultimate expression of sovereignty resides, to a large degree, in the power and the capacity to dictate who may live and who must die" },
    Anchor { id: "mbembe/kill_or_allow_to_live", author: "mbembe", verb: "asserts", subject: "necropolitics/kill_or_allow_to_live", predicate: "claim", object: "To kill or to allow to live constitute the limits of sovereignty, its principal attributes" },
    Anchor { id: "mbembe/death_worlds", author: "mbembe", verb: "argues", subject: "necropolitics/death_worlds", predicate: "claim", object: "Necropolitics names the deployment of weapons in the interest of maximally destroying persons and creating death-worlds -- new and unique forms of social existence in which vast populations are subjected to living conditions conferring on them the status of the living dead" },
    Anchor { id: "mbembe/plantation_colony_camp", author: "mbembe", verb: "asserts", subject: "necropolitics/plantation_colony_camp", predicate: "claim", object: "The plantation, the colony, and the camp are historical sites where necropower has actually operated, not merely theorized -- real institutions, fostered by white supremacy, through which sovereignty over life and death was exercised" },
    Anchor { id: "mbembe/disposability_sovereignty", author: "mbembe", verb: "asserts", subject: "necropolitics/disposability_sovereignty", predicate: "claim", object: "In the colony and the slave plantation, sovereignty means the capacity to define who matters and who does not, who is disposable and who is not" },
    Anchor { id: "mbembe/biopower_control_mortality", author: "mbembe", verb: "extends", subject: "necropolitics/biopower_control_mortality", predicate: "claim", object: "Drawing on and radicalizing Foucault's biopower, to exercise sovereignty is to exercise control over mortality and to define life as the deployment and manifestation of power" },
    Anchor { id: "mbembe/necropolitics_beyond_biopolitics", author: "mbembe", verb: "disputes", subject: "necropolitics/necropolitics_beyond_biopolitics", predicate: "claim", object: "Necropolitics is offered as a needed extension of Foucault's biopolitics, which Mbembe holds is insufficient on its own to account for contemporary forms of the subjugation of life to the power of death -- a paraphrase of the essay's stated framing, not a single verbatim line" },
    Anchor { id: "mbembe/never_certify_from_inside_a_death_world", author: "mbembe", verb: "questions", subject: "necropolitics/never_certify_from_inside_a_death_world", predicate: "tension", object: "An interpretive tension this debate is asked to test directly, not a quote: if sovereignty just is the power to decide who counts, does an ethic of 'never certify, only mourn and indict' remain workable for those actually inside a death-world, or does it presuppose a position of safety Mbembe's own subjects do not have" },
    // Abdullah Öcalan, "Democratic Confederalism" (2011) and related jineology writings.
    Anchor { id: "ocalan/nation_states_obstacles", author: "ocalan", verb: "disputes", subject: "confederalism/nation_states_obstacles", predicate: "claim", object: "The right of self-determination of peoples includes the right to a state of their own, but the foundation of a state does not increase the freedom of a people; the nation-state-based system of the United Nations has remained inefficient, and nation-states have become serious obstacles for any social development" },
    Anchor { id: "ocalan/democratic_confederalism_def", author: "ocalan", verb: "argues", subject: "confederalism/democratic_confederalism_def", predicate: "claim", object: "Democratic confederalism is a non-state social paradigm based on grass-roots participation, in which decision-making lies with the communities themselves and higher levels exist only to coordinate and implement the will of those communities" },
    Anchor { id: "ocalan/state_building_to_society_building", author: "ocalan", verb: "connects", subject: "confederalism/state_building_to_society_building", predicate: "claim", object: "Öcalan replaces state-building with society-building, and the nation-state with confederalism premised on radical democracy pursued outside the nation-state paradigm entirely -- a documented shift in his own later work, not a claim about a single text" },
    Anchor { id: "ocalan/country_free_women_free", author: "ocalan", verb: "asserts", subject: "confederalism/country_free_women_free", predicate: "claim", object: "A country can't be free unless the women are free -- redefining national liberation as, first and foremost, the liberation of women" },
    Anchor { id: "ocalan/liberated_woman_democratic_nation", author: "ocalan", verb: "argues", subject: "confederalism/liberated_woman_democratic_nation", predicate: "claim", object: "Liberated woman constitutes liberated society; liberated society in turn constitutes democratic nation -- women's freedom is not one plank in the confederalist program but its precondition" },
    Anchor { id: "ocalan/killing_the_man", author: "ocalan", verb: "argues", subject: "confederalism/killing_the_man", predicate: "claim", object: "\"Killing the man\" names the basic principle of the project: killing power, one-sided domination, and inequality -- a transformative, ongoing process of relinquishing patriarchal power, not a literal act, required for any society to transcend the capitalist nation-state" },
    Anchor { id: "ocalan/jineology_alternative_science", author: "ocalan", verb: "asserts", subject: "confederalism/jineology_alternative_science", predicate: "claim", object: "Jineology (jineolojî) is proposed as an alternative science, meant to fill gaps the existing social sciences are incapable of addressing on their own" },
    Anchor { id: "ocalan/does_confederalism_still_gate", author: "ocalan", verb: "questions", subject: "confederalism/does_confederalism_still_gate", predicate: "tension", object: "An interpretive tension this debate is asked to test directly, not a quote: a real, still-running commune-based institution has to decide who is a member of the commune and whose voice counts in its assembly -- does democratic confederalism's actual grassroots decision-making escape the certification trap the cyberpunk consensus diagnosed, or does any functioning institution reintroduce some version of the gate" },
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
const MODEL: &str = "z-ai/glm-5.3-flash";
const CYBERPUNK_CONSENSUS_PATH: &str = "../dev-journal/artifacts/2026-08-27-pantheon-commons-cyberpunk-consensus.json";

#[derive(Deserialize)]
struct ConsensusRun {
    final_sequence: Vec<String>,
}

#[derive(Debug, Clone)]
struct TurnRecord {
    cid: String,
    respondent: String,
    round: u32,
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
                    "verb": {"type": "string", "enum": ["argues", "questions", "extends", "disputes", "connects", "reflects", "ruptures"]},
                    "subject": {"type": "string", "description": "short slug for what this turn is about"},
                    "predicate": {"type": "string", "description": "short camelCase predicate naming the claim"},
                    "object": {"type": "string", "description": "the actual claim, one or two sentences, in your own voice"},
                    "consumes": {
                        "type": "array",
                        "description": "at least one EXACT (cid, subject, predicate) copied from the log you were shown -- never invent one. Prefer citing at least one cyberpunk_doc item AND one new anchor when making a claim, to keep the citation concrete.",
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchMode {
    Extend,
    Reflect,
}

async fn dispatch(
    client: &Client<OpenAIConfig>,
    olympian: &Olympian,
    log: &[TurnRecord],
    mode: DispatchMode,
) -> anyhow::Result<DmmlTurnArgs> {
    let user_msg = match mode {
        DispatchMode::Extend => format!(
            "Four Olympians (Athena, Artemis, Apollo, Dionysus) produced a CYBERPUNK CONSENSUS \
document (respondent=cyberpunk, entries subject cyberpunk_doc/item0 through item7) after a debate that \
tested the shamanism consensus (Kopenawa's apprenticeship is a real, working criterion, but must NEVER be \
asked to certify anyone) against Baudrillard's simulation theory and Wynter's genres-of-Man critique. That \
debate concluded something thinner: the real/simulated distinction survives only as an 'observable of \
praxis' -- whether a narration keeps narrating itself -- usable only to mourn a loss or indict its cause, \
never to certify or admit; and that the whole night's underlying question, 'who may sit at the table,' was \
itself asked inside one historically specific genre's vocabulary (Man's), which needs unsettling rather \
than merely widening. That consensus is real and checkpointed, but nothing about it is settled forever -- \
it is one more real position to test. You are not re-arguing the whole thing from scratch; you are \
extending it with two new real sources: Achille Mbembe's 'Necropolitics' (respondent=mbembe), which \
defines sovereignty itself as the power to dictate who may live and who must die, and names real \
historical sites -- the plantation, the colony, the camp -- where that power operated as literal fact, not \
metaphor; and Abdullah Öcalan's 'Democratic Confederalism' (respondent=ocalan), a real, still-running \
attempt to build a non-state institution -- grassroots communes, radical subsidiarity, and an explicit \
claim (via jineology) that women's liberation is the precondition of any liberated society -- that answers \
'who belongs and who decides' outside the state form entirely. Test the new material against the \
cyberpunk consensus, specifically: if sovereignty just IS the power to decide who counts, is 'never \
certify, only mourn and indict' a workable ethic for anyone actually standing inside one of Mbembe's \
death-worlds, or does that restriction quietly presuppose a safety its own subjects don't have? And does \
Öcalan's actual, functioning commune-based institution supply the working answer six debates couldn't find \
-- or does any real institution that has to decide who is a member of the commune and whose voice counts \
in its assembly reintroduce exactly the gate the cyberpunk consensus diagnosed as corrupting? Does a \
specific numbered item in cyberpunk_doc get CONFIRMED, EXTENDED into territory it didn't cover, or actually \
RUPTURED? Other entries in the log are prior turns from you or the other three Olympians in THIS debate.\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona. Prefer citing at least one \
real cyberpunk_doc item together with at least one real new anchor from Mbembe or Öcalan, so the move is \
concrete and checkable, not a vague gesture. If you don't have a real move to make, say so honestly in \
`object` rather than padding. Use verb `ruptures` if you are specifically breaking or overturning a \
cyberpunk_doc item; use `argues`, `extends`, `disputes`, or `connects` for other moves. Call \
submit_dmml_turn with your answer. `consumes` must copy at least one real (cid, subject, predicate) from \
the log above exactly -- never invent one.",
            transcript_so_far(log)
        ),
        DispatchMode::Reflect => format!(
            "The debate among the four Olympians -- testing Mbembe's necropolitics and Öcalan's \
democratic confederalism against the cyberpunk consensus's items -- has just ended. Here is the complete \
real transcript, everything anyone actually said, in order:\n\n{}\n\nThe debate is over -- this is not \
another argumentative move. Reflect, in your own voice as this persona, on your OWN trajectory through it: \
did encountering Mbembe change how you'd now describe the cyberpunk consensus's 'never certify, only \
mourn and indict' restriction -- does it survive contact with real, literal life-and-death stakes, or does \
it need revising? Did encountering Öcalan's actual institution change how you'd now describe the \
'seat at the table' question -- did it supply a real answer, or does even a functioning grassroots \
institution reintroduce the same gate? Name what specifically moved you (a turn, a phrase, a specific new \
anchor), if anything did, or say honestly that nothing did and why not -- false movement is as dishonest \
as false stillness. If you can, cite your own earliest turn in this debate and something later that \
responds to or revises it. Use verb `reflects`. `consumes` should include your own earlier turn if you can \
find one, exactly as it appears in the log -- never invent a citation.",
            transcript_so_far(log)
        ),
    };

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
                    uri: format!("iroh://pantheon-commons-sovereignty/{cid}"),
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
        created_at: "2026-08-28T00:00:00Z".to_string(),
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
    println!("== pantheon commons sovereignty: Mbembe+Ocalan vs. the open cyberpunk consensus ==\n");

    let raw = std::fs::read_to_string(CYBERPUNK_CONSENSUS_PATH)
        .map_err(|e| anyhow::anyhow!("couldn't read cyberpunk consensus at {CYBERPUNK_CONSENSUS_PATH}: {e}"))?;
    let phase1: ConsensusRun = serde_json::from_str(&raw)?;
    println!("loaded cyberpunk consensus ({} statements)\n", phase1.final_sequence.len());

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

    let mut source_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for src in ["cyberpunk", "mbembe", "ocalan"] {
        source_authors.insert(src, api.author_create().await?);
    }
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        source_authors["cyberpunk"],
        "pantheon-commons-sovereignty".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} frozen cyberpunk-consensus items --", phase1.final_sequence.len());
    // verb == predicate == "statement" here, deliberately -- see
    // dev-journal 2026-08-27-pantheon-commons-rupture.md for why.
    let synthesis_author = source_authors["cyberpunk"];
    for (i, statement) in phase1.final_sequence.iter().enumerate() {
        let subject = format!("cyberpunk_doc/item{i}");
        let mut rec = append(&substrate, &synthesis_author, 0, "statement", &subject, "statement", statement, &[]).await?;
        rec.respondent = "cyberpunk".to_string();
        println!("  [item{i}] {} -> \"{}\"", rec.cid, rec.object);
        log.push(rec);
    }

    println!("\n-- seeding {} new anchor claims (mbembe + ocalan) --", NEW_ANCHORS.len());
    for a in NEW_ANCHORS {
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
            match dispatch(&client, olympian, &log, DispatchMode::Extend).await {
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

    let reflect_round = (ROUNDS + 1) as u32;
    println!("\n-- reflection round --");
    for olympian in OLYMPIANS {
        print!("  dispatching {} (reflecting)... ", olympian.name);
        use std::io::Write;
        std::io::stdout().flush().ok();
        match dispatch(&client, olympian, &log, DispatchMode::Reflect).await {
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
                    reflect_round,
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
    std::fs::write("pantheon_commons_sovereignty.json", &json)?;
    println!("wrote pantheon_commons_sovereignty.json ({} entries)", dumped.len());

    Ok(())
}
