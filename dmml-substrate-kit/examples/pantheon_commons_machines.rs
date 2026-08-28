//! Phase 7: "does the negative gate exit power" -- Deleuze/Guattari and
//! Foucault against the sovereignty consensus. This round is different
//! from every prior extension in one specific way: DMML's own ontology
//! (desiring-machines, the commit graph's consumes/produces model,
//! petitions as transindividual claims) is explicitly modeled on D&G's
//! desiring-production framework (see this paper's companion piece,
//! "DMML as a Desiring-Production Ontology"). Bringing D&G into the
//! DEBATE CONTENT, not just the substrate's design, means the pantheon
//! is now reasoning about the same apparatus it runs on.
//!
//! The sovereignty consensus's final move was that a legitimate commune
//! must be able to dissolve -- to cease being the one who permits, even
//! of itself -- as the only way to avoid rebuilding a gate. Foucault's
//! whole argument in Discipline and Punish and The History of Sexuality
//! is that this is close to a category error: power is not a possession
//! an institution can renounce, it is relational, produced at every
//! point ("power is everywhere; not because it embraces everything, but
//! because it comes from everywhere"), and discipline's most effective
//! form doesn't coerce, it trains subjects to monitor themselves (docile
//! bodies, the Panopticon's "state of conscious and permanent
//! visibility"). A commune that asks members to bring their own claims
//! forward without a pre-screener may not have exited power at all -- it
//! may have relocated it into self-surveillance, which is arguably a
//! MORE effective disciplinary technique than an external gate, not an
//! escape from one.
//!
//! Deleuze and Guattari cut the other way: is the negative gate (open,
//! rootless, "no one may adjudicate before the claim occurs") actually a
//! rhizome -- decentered, no beginning or end, always in the middle --
//! or has the whole pantheon's own procedure (a fixed cast of four
//! Olympians, a tool schema, a ratification vote) been an arborescent,
//! coded State apparatus the entire time, running underneath a rhetoric
//! of openness? And is Öcalan's "ordering, not a criterion" (whose voice
//! was erased first) itself a war machine -- exterior to the State
//! apparatus, "coming from elsewhere" -- or a deterritorialized flow
//! that the debate's own consensus mechanism (four fixed voters, a
//! majority-rules ratification) will reterritorialize the moment it is
//! adopted?
//!
//! Neither D&G nor Foucault has a Power Explained dramatis-personae
//! profile (real gap, noted in the dev-journal and paper update); all
//! are canonical, extremely well-documented figures, and every claim
//! below is either a verified direct quote (checked live via web search
//! on 2026-08-28 before this file was written) or an explicitly labeled
//! paraphrase of a well-documented argument.
//!
//! Structurally: same frozen-prior-plus-new-anchors pattern as every
//! extension run, this time against the SOVEREIGNTY run's own consensus
//! (`pantheon_commons_sovereignty_consensus.rs`'s output).

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
    // Gilles Deleuze and Felix Guattari, Anti-Oedipus (1972) and A Thousand Plateaus (1980).
    Anchor { id: "dg/desiring_machines", author: "deleuzeguattari", verb: "asserts", subject: "desire/desiring_machines", predicate: "claim", object: "Everywhere it is machines -- real ones, not figurative ones: machines driving other machines, machines being driven by other machines, with all the necessary couplings and connections" },
    Anchor { id: "dg/desire_is_a_machine", author: "deleuzeguattari", verb: "argues", subject: "desire/desire_is_a_machine", predicate: "claim", object: "Desire is a machine, and the object of desire is another machine connected to it -- desire is not lack but production, connection, and flow, not a want directed at a missing object" },
    Anchor { id: "dg/body_without_organs", author: "deleuzeguattari", verb: "stipulates", subject: "desire/body_without_organs", predicate: "distinction", object: "The body without organs is not opposed to the organs; rather, the body without organs and its true organs are opposed to the organism, the organic organization and hierarchy of the organs" },
    Anchor { id: "dg/rhizome_always_middle", author: "deleuzeguattari", verb: "asserts", subject: "rhizome/rhizome_always_middle", predicate: "claim", object: "A rhizome has no beginning or end; it is always in the middle, between things, interbeing, intermezzo -- opposed to the arborescent, rooted structure of a tree with a single point of origin" },
    Anchor { id: "dg/war_machine_exterior_to_state", author: "deleuzeguattari", verb: "asserts", subject: "nomadology/war_machine_exterior_to_state", predicate: "claim", object: "The war machine is exterior to the State apparatus; it is irreducible to the State, outside its sovereignty and prior to its law, and comes from elsewhere" },
    Anchor { id: "dg/smooth_striated_space", author: "deleuzeguattari", verb: "stipulates", subject: "nomadology/smooth_striated_space", predicate: "distinction", object: "Smooth space and striated space, nomad space and sedentary space, are distinct modes of occupying territory: the striated codes and fixes points and paths, the smooth space is occupied without being counted" },
    Anchor { id: "dg/molar_molecular_segmentarity", author: "deleuzeguattari", verb: "argues", subject: "micropolitics/molar_molecular_segmentarity", predicate: "claim", object: "Every society, and every individual, is plied by two segmentarities simultaneously, one molar (rigid, coded, institutional) and one molecular (supple, fluid, escaping codification) -- macropolitics and micropolitics operate at once, neither reducible to the other" },
    Anchor { id: "dg/deterritorialization", author: "deleuzeguattari", verb: "extends", subject: "desire/deterritorialization", predicate: "claim", object: "Flows of desire and of capital undergo deterritorialization (breaking free of a fixed code or territory) and reterritorialization (being recaptured onto a new one) as a constant, ongoing process, not a single one-way liberation -- a documented structural claim across Anti-Oedipus, not a single verbatim line" },
    // Michel Foucault, Discipline and Punish (1975) and The History of Sexuality, Vol. 1 (1976).
    Anchor { id: "foucault/power_is_everywhere", author: "foucault", verb: "asserts", subject: "power/power_is_everywhere", predicate: "claim", object: "Power is produced from one moment to the next, at every point, or rather in every relation from one point to another; power is everywhere -- not because it embraces everything, but because it comes from everywhere" },
    Anchor { id: "foucault/power_not_institution", author: "foucault", verb: "disputes", subject: "power/power_not_institution", predicate: "claim", object: "Power is not an institution, and not a structure; neither is it a certain strength we are endowed with; it is the name that one attributes to a complex strategical situation in a particular society" },
    Anchor { id: "foucault/panopticon_permanent_visibility", author: "foucault", verb: "argues", subject: "discipline/panopticon_permanent_visibility", predicate: "claim", object: "The major effect of the Panopticon is to induce in the inmate a state of conscious and permanent visibility that assures the automatic functioning of power, so a real subjection is born mechanically from a fictitious relation" },
    Anchor { id: "foucault/docile_bodies", author: "foucault", verb: "asserts", subject: "discipline/docile_bodies", predicate: "claim", object: "A body is docile that may be subjected, used, transformed and improved -- discipline produces subjected and practiced bodies, 'docile' bodies, through methods that make possible the meticulous control of the operations of the body" },
    Anchor { id: "foucault/where_power_resistance", author: "foucault", verb: "asserts", subject: "power/where_power_resistance", predicate: "claim", object: "Where there is power, there is resistance, and yet, or rather consequently, this resistance is never in a position of exteriority in relation to power" },
    Anchor { id: "foucault/power_produces_knowledge", author: "foucault", verb: "connects", subject: "power/power_produces_knowledge", predicate: "claim", object: "Power produces knowledge; power and knowledge directly imply one another -- there is no power relation without the correlative constitution of a field of knowledge, nor any knowledge that does not presuppose and constitute power relations" },
    Anchor { id: "foucault/biopower_capitalism", author: "foucault", verb: "argues", subject: "power/biopower_capitalism", predicate: "claim", object: "This bio-power was without question an indispensable element in the development of capitalism, which would not have been possible without the controlled insertion of bodies into the machinery of production and the adjustment of the phenomena of population to economic processes" },
    Anchor { id: "foucault/discipline_makes_individuals", author: "foucault", verb: "argues", subject: "discipline/discipline_makes_individuals", predicate: "claim", object: "Discipline 'makes' individuals; it is the specific technique of a power that regards individuals both as objects and as instruments of its exercise, not a triumphant power that can rely on its own excess but a modest, suspicious power that functions like an economy" },
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
const SOVEREIGNTY_CONSENSUS_PATH: &str = "../dev-journal/artifacts/2026-08-28-pantheon-commons-sovereignty-consensus.json";

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
                        "description": "at least one EXACT (cid, subject, predicate) copied from the log you were shown -- never invent one. Prefer citing at least one sovereignty_doc item AND one new anchor when making a claim, to keep the citation concrete.",
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
            "Four Olympians (Athena, Artemis, Apollo, Dionysus) produced a SOVEREIGNTY CONSENSUS \
document (respondent=sovereignty, entries subject sovereignty_doc/item0 through item8) after a debate that \
tested the cyberpunk consensus's 'never certify, only mourn and indict' restriction against Achille \
Mbembe's necropolitics and Abdullah Ocalan's democratic confederalism. That debate found that every \
positive repair to the restriction (an internal auditor, a scheduled clock, a testimony archive, a public \
ledger of absences) quietly reintroduced an examiner, and converged on a NEGATIVE form instead: anyone who \
claims erasure may convene the commune's process, but no one -- not the commune, not its own trusted \
instruments -- may adjudicate the claim before it occurs; and that a legitimate commune must be able to \
dissolve, to cease being the one who permits, even of itself. That consensus is real and checkpointed, but \
nothing about it is settled forever -- it is one more real position to test. You are not re-arguing the \
whole thing from scratch; you are extending it with two new real sources: Deleuze and Guattari \
(respondent=deleuzeguattari), on desire as productive machinic connection rather than lack, the rhizome as \
a decentered structure with no beginning or root, and the war machine as exterior to the State apparatus, \
coming from elsewhere; and Michel Foucault (respondent=foucault), on power as relational and produced at \
every point rather than a possession an institution can hold or renounce, and on discipline's most \
effective form as training subjects to monitor themselves (docile bodies, the Panopticon's permanent \
visibility) rather than coercing them externally. Test the new material against the sovereignty consensus, \
specifically: when the consensus says a legitimate commune must be able to 'dissolve, cease being the one \
who permits, even of itself,' is that a real exit from power, or does Foucault show it is a category error \
-- power cannot be renounced because it was never held as a possession in the first place, and a commune \
that asks members to bring their own claims forward without a pre-screener may just be relocating power \
into self-surveillance, arguably MORE effective discipline than an external gate, not an escape from one? \
And is the negative gate itself (open, rootless, no adjudication before the claim occurs) actually a \
rhizome in Deleuze and Guattari's sense, or has this whole pantheon's own procedure -- a fixed cast of four \
Olympians, a tool schema, a majority-rules ratification vote -- been an arborescent, coded State apparatus \
the entire time, running underneath a rhetoric of openness? Is Ocalan's 'ordering, not a criterion' (whose \
voice was erased first) a war machine exterior to the State, or a deterritorialized flow this very \
consensus mechanism will reterritorialize the moment it adopts it as a rule? Does a specific numbered item \
in sovereignty_doc get CONFIRMED, EXTENDED into territory it didn't cover, or actually RUPTURED? Other \
entries in the log are prior turns from you or the other three Olympians in THIS debate.\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona. Prefer citing at least one \
real sovereignty_doc item together with at least one real new anchor from Deleuze/Guattari or Foucault, so \
the move is concrete and checkable, not a vague gesture. If you don't have a real move to make, say so \
honestly in `object` rather than padding. Use verb `ruptures` if you are specifically breaking or \
overturning a sovereignty_doc item; use `argues`, `extends`, `disputes`, or `connects` for other moves. \
Call submit_dmml_turn with your answer. `consumes` must copy at least one real (cid, subject, predicate) \
from the log above exactly -- never invent one.",
            transcript_so_far(log)
        ),
        DispatchMode::Reflect => format!(
            "The debate among the four Olympians -- testing Deleuze/Guattari's desiring-production and \
war-machine framework and Foucault's relational, self-disciplining account of power against the \
sovereignty consensus's items -- has just ended. Here is the complete real transcript, everything anyone \
actually said, in order:\n\n{}\n\nThe debate is over -- this is not another argumentative move. Reflect, in \
your own voice as this persona, on your OWN trajectory through it: did encountering Foucault change how \
you'd now describe the sovereignty consensus's claim that a commune can 'dissolve, cease being the one who \
permits' -- do you now think that is a real exit from power, or has Foucault shown the whole night's search \
for a gate-free institution was always going to relocate power rather than escape it? Did encountering \
Deleuze and Guattari change how you'd now describe this very pantheon's own procedure -- is it a rhizome, \
or has it been a coded, arborescent apparatus the whole time? Name what specifically moved you (a turn, a \
phrase, a specific new anchor), if anything did, or say honestly that nothing did and why not -- false \
movement is as dishonest as false stillness. If you can, cite your own earliest turn in this debate and \
something later that responds to or revises it. Use verb `reflects`. `consumes` should include your own \
earlier turn if you can find one, exactly as it appears in the log -- never invent a citation.",
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
                    uri: format!("iroh://pantheon-commons-machines/{cid}"),
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
    println!("== pantheon commons machines: Deleuze/Guattari+Foucault vs. the open sovereignty consensus ==\n");

    let raw = std::fs::read_to_string(SOVEREIGNTY_CONSENSUS_PATH)
        .map_err(|e| anyhow::anyhow!("couldn't read sovereignty consensus at {SOVEREIGNTY_CONSENSUS_PATH}: {e}"))?;
    let phase1: ConsensusRun = serde_json::from_str(&raw)?;
    println!("loaded sovereignty consensus ({} statements)\n", phase1.final_sequence.len());

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
    for src in ["sovereignty", "deleuzeguattari", "foucault"] {
        source_authors.insert(src, api.author_create().await?);
    }
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        source_authors["sovereignty"],
        "pantheon-commons-machines".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} frozen sovereignty-consensus items --", phase1.final_sequence.len());
    // verb == predicate == "statement" here, deliberately -- see
    // dev-journal 2026-08-27-pantheon-commons-rupture.md for why.
    let synthesis_author = source_authors["sovereignty"];
    for (i, statement) in phase1.final_sequence.iter().enumerate() {
        let subject = format!("sovereignty_doc/item{i}");
        let mut rec = append(&substrate, &synthesis_author, 0, "statement", &subject, "statement", statement, &[]).await?;
        rec.respondent = "sovereignty".to_string();
        println!("  [item{i}] {} -> \"{}\"", rec.cid, rec.object);
        log.push(rec);
    }

    println!("\n-- seeding {} new anchor claims (deleuze+guattari + foucault) --", NEW_ANCHORS.len());
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
    std::fs::write("pantheon_commons_machines.json", &json)?;
    println!("wrote pantheon_commons_machines.json ({} entries)", dumped.len());

    Ok(())
}
