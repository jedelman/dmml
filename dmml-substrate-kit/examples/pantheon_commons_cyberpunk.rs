//! Phase 5: "the cyberpunk turn," per Jason's plan: "then we get into
//! cyberpunk - Baudrillard and Wynter." Jean Baudrillard's Simulacra and
//! Simulation (hyperreality, the precession of simulacra -- the model
//! precedes and produces the "real" rather than representing it) and
//! Sylvia Wynter's "Unsettling the Coloniality of Being" (the human is
//! not one biological given but a series of historically overrepresented
//! "genres of Man," Man1/Man2, homo oeconomicus) are natural next moves
//! against the shamanism run's own conclusion: Kopenawa's real,
//! rigorous, non-certifying criterion. Baudrillard forces the question
//! of whether "real" and "certified" are still even a stable pair once
//! simulation has replaced the referent; Wynter forces the question of
//! whether the whole "who may sit at the table" framing already
//! presupposes one historically specific, overrepresented genre of the
//! human as the only kind of subject capable of holding a seat at all.
//! Wynter is a real `G-DP-*` profile in Power Explained (`G-DP-012`);
//! Baudrillard is not yet profiled there but is a natural extension of
//! the corpus's existing engagement with simulation, spectacle, and mass
//! culture (Debord, Adorno's culture industry, both cited earlier
//! tonight).
//!
//! Structurally: same frozen-prior-plus-new-anchors pattern as every
//! extension run, this time against the SHAMANISM run's own consensus
//! (`pantheon_commons_shamanism_consensus.rs`'s output) -- which
//! concluded a real criterion exists (Kopenawa's apprenticeship) but
//! must never be asked to certify, on pain of becoming "a copy
//! performing for the certifier." `DispatchMode::Extend` invites
//! confirmation, extension, or rupture of specific shamanism-consensus
//! items using this round's two new sources.

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
// discipline as every anchor in this project.
const NEW_ANCHORS: &[Anchor] = &[
    // Jean Baudrillard, Simulacra and Simulation (1981).
    Anchor { id: "baudrillard/precession_of_simulacra", author: "baudrillard", verb: "argues", subject: "simulacra/precession_of_simulacra", predicate: "claim", object: "The precession of simulacra names a reversal in which the model precedes the event -- simulation comes first, and reality arranges itself to conform to the model rather than the model representing a prior reality" },
    Anchor { id: "baudrillard/map_precedes_territory", author: "baudrillard", verb: "asserts", subject: "simulacra/map_precedes_territory", predicate: "claim", object: "The territory no longer precedes the map, nor survives it; henceforth it is the map that precedes the territory, the map that engenders the territory" },
    Anchor { id: "baudrillard/desert_of_the_real", author: "baudrillard", verb: "asserts", subject: "simulacra/desert_of_the_real", predicate: "claim", object: "What remains of the real once the model has replaced it is a desert -- the desert of the real itself, vestiges scattered rather than a living territory" },
    Anchor { id: "baudrillard/disneyland_conceals_real", author: "baudrillard", verb: "asserts", subject: "simulacra/disneyland_conceals_real", predicate: "claim", object: "Disneyland is there to conceal the fact that it is the real country, all of real America, which is Disneyland" },
    Anchor { id: "baudrillard/disneyland_infantile", author: "baudrillard", verb: "argues", subject: "simulacra/disneyland_infantile", predicate: "claim", object: "Disneyland is presented as an imaginary, infantile world in order to make us believe that the adults are elsewhere, in the real world, and to conceal the fact that real childishness is everywhere" },
    Anchor { id: "baudrillard/signs_refer_to_signs", author: "baudrillard", verb: "disputes", subject: "simulacra/signs_refer_to_signs", predicate: "claim", object: "In the order of simulation, signs no longer refer to any reality at all; they refer only to other signs, forming a closed self-referential system rather than a representational one" },
    Anchor { id: "baudrillard/four_orders_of_simulacra", author: "baudrillard", verb: "argues", subject: "simulacra/four_orders_of_simulacra", predicate: "claim", object: "Representation passes through four successive orders -- faithful reflection, distortion of a reality, disguising the absence of a reality, and pure simulation bearing no relation to any reality at all -- and the hyperreal names specifically this fourth order" },
    Anchor { id: "baudrillard/simulation_vs_dissimulation", author: "baudrillard", verb: "stipulates", subject: "simulacra/simulation_vs_dissimulation", predicate: "distinction", object: "Dissimulation is feigning not to have what one has, leaving the reality principle intact; simulation is feigning to have what one doesn't have, which threatens the very difference between true and false, real and imaginary" },
    // Sylvia Wynter, "Unsettling the Coloniality of Being/Power/Truth/Freedom" (2003) and On Being Human as Praxis.
    Anchor { id: "wynter/man_as_overrepresented", author: "wynter", verb: "disputes", subject: "coloniality/man_as_overrepresented", predicate: "claim", object: "Man, specifically the post-1492 Western bourgeois figure, has been overrepresented as if it were the human as such -- a substitution Wynter's whole project works to unsettle and denaturalize" },
    Anchor { id: "wynter/man1_political_subject", author: "wynter", verb: "argues", subject: "coloniality/man1_political_subject", predicate: "claim", object: "Man1 emerges during the Renaissance as a political, rational subject defined against a religiously-coded Other, the idolator or heathen -- the first post-medieval reinvention of the human" },
    Anchor { id: "wynter/man2_homo_oeconomicus", author: "wynter", verb: "argues", subject: "coloniality/man2_homo_oeconomicus", predicate: "claim", object: "Man2, the Enlightenment's biocentric, economic figure -- homo oeconomicus -- supersedes Man1 without displacing its logic of hierarchy, redefining the human via natural selection, race, and market rationality" },
    Anchor { id: "wynter/genres_of_being_human", author: "wynter", verb: "argues", subject: "coloniality/genres_of_being_human", predicate: "claim", object: "The human is not a fixed biological given but exists historically as a plurality of self-inventing genres of being human, each institutionalized through its own origin narrative, none of which is simply the human as such" },
    Anchor { id: "wynter/coloniality_of_being", author: "wynter", verb: "argues", subject: "coloniality/coloniality_of_being", predicate: "claim", object: "The coloniality of power has as its condition of possibility a deeper coloniality of being -- the West's own overrepresented genre of Man determining who counts as fully human at all" },
    Anchor { id: "wynter/homo_narrans", author: "wynter", verb: "argues", subject: "praxis/homo_narrans", predicate: "claim", object: "Humans are a hybrid-auto-instituting-languaging-storytelling species, homo narrans -- the capacity for origin-narrative and myth is not decoration on top of biology but constitutive of what makes humans the kind of beings they are" },
    Anchor { id: "wynter/demonic_ground", author: "wynter", verb: "argues", subject: "coloniality/demonic_ground", predicate: "claim", object: "Demonic ground names a standpoint outside the current governing figuration of the human as Man, from which that figuration's own contingency becomes visible and other genres of being human become imaginable" },
    Anchor { id: "wynter/being_human_as_praxis", author: "wynter", verb: "disputes", subject: "praxis/being_human_as_praxis", predicate: "claim", object: "Being human is not a fixed state or biological fact to be described but a praxis -- an ongoing, collective, always-unfinished action of self-making that could, in principle, be redone differently" },
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
const SHAMANISM_CONSENSUS_PATH: &str = "../dev-journal/artifacts/2026-08-27-pantheon-commons-shamanism-consensus.json";

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
                        "description": "at least one EXACT (cid, subject, predicate) copied from the log you were shown -- never invent one. Prefer citing at least one shamanism_doc item AND one new anchor when making a claim, to keep the citation concrete.",
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
            "Four Olympians (Athena, Artemis, Apollo, Dionysus) produced a SHAMANISM CONSENSUS \
document (respondent=shamanism, entries subject shamanism_doc/item0 through item10) after a debate that \
tested the embodiment consensus (no criterion for an open rule-making seat survived; the demand for one \
was itself a form of armor) against Eliade, Taussig, Ginzburg, and Kopenawa. That debate found a real, \
working criterion after all -- Kopenawa's documented shamanic apprenticeship -- but concluded it must \
NEVER be asked to certify anyone, because doing so converts it into 'a copy performing for the \
certifier' (a corruption Ginzburg's benandanti, real people reshaped by a century of their own \
persecutors' interrogation, prove historically, not just theoretically). That consensus is real and \
checkpointed, but nothing about it is settled forever -- it is one more real position to test. You are \
not re-arguing the whole thing from scratch; you are extending it with two new real sources: Jean \
Baudrillard's Simulacra and Simulation (respondent=baudrillard), on the precession of simulacra -- the \
claim that in a hyperreal order, the model precedes and produces the real rather than representing a \
prior reality, so that signs no longer refer to anything except other signs; and Sylvia Wynter's \
'Unsettling the Coloniality of Being' (respondent=wynter), on 'Man' as one historically specific, \
overrepresented genre of the human (Man1, the Renaissance political subject; Man2, homo oeconomicus) \
mistaken for the human as such, and on being human as an ongoing praxis rather than a fixed fact. Test \
the new material against the shamanism consensus, specifically: if simulation has replaced the referent, \
does 'real, working criterion' (Kopenawa's apprenticeship) even remain a coherent category, or is the \
whole real/simulated distinction the consensus relies on itself a Man2-era, homo-oeconomicus habit of \
mind Wynter would ask us to denaturalize? Does Wynter's genres-of-Man argument suggest the entire 'who \
may sit at the table' framing, across every prior debate tonight, already smuggled in one historically \
specific genre of the human as the only kind of subject capable of holding a seat at all -- making the \
seat question not merely unanswerable but the wrong question from the start, asked in the wrong \
vocabulary? Does a specific numbered item in shamanism_doc get CONFIRMED, EXTENDED into territory it \
didn't cover, or actually RUPTURED? Other entries in the log are prior turns from you or the other three \
Olympians in THIS debate.\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona. Prefer citing at least one \
real shamanism_doc item together with at least one real new anchor from Baudrillard or Wynter, so the \
move is concrete and checkable, not a vague gesture. If you don't have a real move to make, say so \
honestly in `object` rather than padding. Use verb `ruptures` if you are specifically breaking or \
overturning a shamanism_doc item; use `argues`, `extends`, `disputes`, or `connects` for other moves. \
Call submit_dmml_turn with your answer. `consumes` must copy at least one real (cid, subject, predicate) \
from the log above exactly -- never invent one.",
            transcript_so_far(log)
        ),
        DispatchMode::Reflect => format!(
            "The debate among the four Olympians -- testing Baudrillard's simulation theory and \
Wynter's genres-of-Man critique against the shamanism consensus's items -- has just ended. Here is the \
complete real transcript, everything anyone actually said, in order:\n\n{}\n\nThe debate is over -- this \
is not another argumentative move. Reflect, in your own voice as this persona, on your OWN trajectory \
through it: did encountering Baudrillard and Wynter change how you'd now describe the shamanism \
consensus's finding -- do you now think the real/simulated distinction it relies on still holds, or has \
Wynter's critique of Man as an overrepresented genre shown the whole 'seat at the table' question, across \
every debate tonight, was asked from inside one historically specific vocabulary that should itself be \
unsettled? Name what specifically moved you (a turn, a phrase, a specific new anchor), if anything did, \
or say honestly that nothing did and why not -- false movement is as dishonest as false stillness. If \
you can, cite your own earliest turn in this debate and something later that responds to or revises it. \
Use verb `reflects`. `consumes` should include your own earlier turn if you can find one, exactly as it \
appears in the log -- never invent a citation.",
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
                    uri: format!("iroh://pantheon-commons-cyberpunk/{cid}"),
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
    println!("== pantheon commons cyberpunk: Baudrillard+Wynter vs. the open shamanism consensus ==\n");

    let raw = std::fs::read_to_string(SHAMANISM_CONSENSUS_PATH)
        .map_err(|e| anyhow::anyhow!("couldn't read shamanism consensus at {SHAMANISM_CONSENSUS_PATH}: {e}"))?;
    let phase1: ConsensusRun = serde_json::from_str(&raw)?;
    println!("loaded shamanism consensus ({} statements)\n", phase1.final_sequence.len());

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
    for src in ["shamanism", "baudrillard", "wynter"] {
        source_authors.insert(src, api.author_create().await?);
    }
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        source_authors["shamanism"],
        "pantheon-commons-cyberpunk".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} frozen shamanism-consensus items --", phase1.final_sequence.len());
    // verb == predicate == "statement" here, deliberately: the rupture run
    // (pantheon_commons_rupture.rs) found models repeatedly citing the verb
    // field ("ratifiedAs") as if it were the predicate field ("statement"),
    // a real, root-caused schema ambiguity from using two different strings
    // for those fields on frozen prior-consensus items -- see dev-journal
    // 2026-08-27-pantheon-commons-rupture.md. Using the same string for
    // both fields here makes that specific confusion harmless.
    let synthesis_author = source_authors["shamanism"];
    for (i, statement) in phase1.final_sequence.iter().enumerate() {
        let subject = format!("shamanism_doc/item{i}");
        let mut rec = append(&substrate, &synthesis_author, 0, "statement", &subject, "statement", statement, &[]).await?;
        rec.respondent = "shamanism".to_string();
        println!("  [item{i}] {} -> \"{}\"", rec.cid, rec.object);
        log.push(rec);
    }

    println!("\n-- seeding {} new anchor claims (reich + plotkin) --", NEW_ANCHORS.len());
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
    std::fs::write("pantheon_commons_cyberpunk.json", &json)?;
    println!("wrote pantheon_commons_cyberpunk.json ({} entries)", dumped.len());

    Ok(())
}
