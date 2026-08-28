//! Phase 8: "constituent power" -- Hardt and Negri arrive not as one
//! more source to test against a frozen consensus, but as the occasion
//! for a genuinely different question: can the four Olympians choose,
//! individually, which of their own four rhizome-provocation proposals
//! (Athena's web, Artemis's hunt, Apollo's lyre, Dionysus's sparagmos)
//! they actually want to be governed by -- not as a group vote imposing
//! one winner on all four, but as each agent adopting their own rule,
//! simultaneously, with no meta-authority reconciling the differences?
//! Jason's framing: none of the four protocols are built into DMML's
//! ontology (fixed order, numbered items, majority ratification are all
//! this harness's own scaffolding, not the commit graph's), so nothing
//! stops the agents from actually choosing. Hardt and Negri's Multitude
//! supplies the exact vocabulary for whether this can work at all: "the
//! multitude is composed of singularities that act in common" -- not
//! unified into one will, not incoherent either, but joined by what its
//! constituents share without being reduced to sameness. Empire's own
//! argument supplies the shadow case: a decentered, deterritorializing
//! network power that "has no outside" is not automatically liberatory
//! just because it lacks a single throne -- Empire itself is
//! non-sovereign-in-form. So the debate this file runs is a real test:
//! does letting each Olympian choose their own governing rule produce
//! Hardt and Negri's multitude (plural, coherent through the common) or
//! does it just reproduce Empire's own decentered, still-total capture
//! at the pantheon's own scale?
//!
//! Mechanically, three things are genuinely different from every prior
//! file, not merely discussed: (1) a real choice round precedes the
//! debate, where each Olympian picks or hybridizes a protocol for
//! themselves, unprompted to coordinate with the others; (2) dispatch
//! order is reshuffled every round rather than fixed
//! Athena-Artemis-Apollo-Dionysus, and any agent whose choice-round text
//! commits to Artemis's hunt is prompted each turn to name a citation
//! target it is pursuing; (3) any agent who commits to Dionysus's
//! sparagmos has their turn's PUBLIC attribution (the checkpointed
//! `respondent` field) reassigned by lot to a different persona name
//! after the turn is drafted -- true authorship is still printed to
//! stdout for transparency, but the graph itself records the mask, not
//! the mouth. There is deliberately no consensus/ratification stage in
//! this file: ratification is computed programmatically at the end as
//! "kept alive by use" (cited at least twice by distinct respondents),
//! Athena's own proposed mechanism, applied because it is the one the
//! four could not have refused without falling back on the vote they
//! all just spent seven phases undermining. Full concurrent (non-round)
//! dispatch, Apollo's most radical ask, is NOT implemented here --
//! reshuffled-per-round is the practical proxy; this simplification is
//! named honestly rather than claimed as more than it is.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

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

// The 16 new anchors, all Hardt and Negri (Empire, 2000; Multitude,
// 2004) -- always co-authored, so treated as one source, same
// convention as deleuzeguattari. Every claim is either a verified
// direct quote or an explicitly labeled paraphrase; the two most
// load-bearing ("singularities that act in common," "sole constituent
// power") verified live via web search on 2026-08-28 before this file
// was written.
const NEW_ANCHORS: &[Anchor] = &[
    // Empire (2000).
    Anchor { id: "hn/empire_no_outside", author: "hardtnegri", verb: "asserts", subject: "empire/empire_no_outside", predicate: "claim", object: "The new imperial power is exercised globally and has no outside -- it establishes no territorial center of power and does not rely on fixed boundaries or barriers" },
    Anchor { id: "hn/empire_suspends_history", author: "hardtnegri", verb: "argues", subject: "empire/empire_suspends_history", predicate: "claim", object: "Empire presents itself not as a historical regime originating in conquest but rather as an order that effectively suspends history and thereby fixes the existing state of affairs for eternity" },
    Anchor { id: "hn/empire_decentered_deterritorializing", author: "hardtnegri", verb: "asserts", subject: "empire/empire_decentered_deterritorializing", predicate: "claim", object: "Empire is a decentered and deterritorializing apparatus of rule that progressively incorporates the entire global realm within its open, expanding frontiers" },
    Anchor { id: "hn/empire_network_power", author: "hardtnegri", verb: "extends", subject: "empire/empire_network_power", predicate: "claim", object: "Sovereignty under Empire takes the form of a network power, distinct in kind from the modern imperialisms of competing nation-states -- a documented structural claim across the book, not a single verbatim line" },
    Anchor { id: "hn/empire_biopolitical_production", author: "hardtnegri", verb: "argues", subject: "empire/empire_biopolitical_production", predicate: "claim", object: "Production today is biopolitical: it produces not only material goods but social life itself, including relationships, communication, and forms of life -- a documented structural claim, not a single verbatim line" },
    Anchor { id: "hn/multitude_as_productive_force", author: "hardtnegri", verb: "disputes", subject: "empire/multitude_as_productive_force", predicate: "claim", object: "The multitude is the real productive, creative force of social life, while Empire is merely an apparatus of capture that lives off the vitality of the multitude" },
    Anchor { id: "hn/empire_smooth_space", author: "hardtnegri", verb: "asserts", subject: "empire/empire_smooth_space", predicate: "claim", object: "Empire operates increasingly on smooth space, deploying not fixed boundaries but modulating networks of command that adapt continuously rather than rule from a single center" },
    Anchor { id: "hn/empire_permanent_intervention", author: "hardtnegri", verb: "argues", subject: "empire/empire_permanent_intervention", predicate: "claim", object: "Empire justifies its interventions as moral police actions rather than conquest, in the name of a permanent, universal order it claims already exists rather than one still being imposed -- a documented structural claim about Empire's self-justification, not a single verbatim line" },
    // Multitude (2004).
    Anchor { id: "hn/multitude_singularities_common", author: "hardtnegri", verb: "asserts", subject: "multitude/multitude_singularities_common", predicate: "claim", object: "The multitude is composed of singularities that act in common -- an active social subject that acts on the basis of what the singularities share, not on the basis of their identity or sameness" },
    Anchor { id: "hn/multitude_not_unified", author: "hardtnegri", verb: "stipulates", subject: "multitude/multitude_not_unified", predicate: "distinction", object: "The multitude is not unified into a single will, but it is not anarchic or incoherent either -- it is multiple and plural, joined by the power of what its constituents hold in common" },
    Anchor { id: "hn/multitude_democracy", author: "hardtnegri", verb: "argues", subject: "multitude/multitude_democracy", predicate: "claim", object: "The multitude is the only form of political subjectivity capable of realizing democracy for what it truly is: the rule of everyone by everyone" },
    Anchor { id: "hn/constituent_power", author: "hardtnegri", verb: "asserts", subject: "multitude/constituent_power", predicate: "claim", object: "The multitude is the sole constituent power, the only creative agent that produces both itself and its historic antagonist, constituted power -- capital, sovereignty, Empire" },
    Anchor { id: "hn/multitude_vs_the_people", author: "hardtnegri", verb: "disputes", subject: "multitude/multitude_vs_the_people", predicate: "claim", object: "'The people' has traditionally been represented as a unity reducible to a single will or sovereign body; the multitude resists exactly this reduction to sameness, remaining irreducibly plural" },
    Anchor { id: "hn/multitude_network_form", author: "hardtnegri", verb: "connects", subject: "multitude/multitude_network_form", predicate: "claim", object: "The multitude's own political form is imagined as a network: distributed, horizontal, composed of irreducible differences, in explicit contrast to the pyramidal, command-based forms of traditional sovereignty" },
    Anchor { id: "hn/multitude_war_democracy_question", author: "hardtnegri", verb: "questions", subject: "multitude/multitude_war_democracy_question", predicate: "claim", object: "Multitude's central question, named in its own subtitle (War and Democracy in the Age of Empire): in an age of global, generalized war, is democracy still possible, and could the multitude be the political subject capable of it" },
    Anchor { id: "hn/common_not_uniform", author: "hardtnegri", verb: "argues", subject: "multitude/common_not_uniform", predicate: "claim", object: "The common is not sameness or uniformity but what is produced through communication, collaboration, and encounter among real differences -- the common requires differences to persist, not dissolve" },
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
const MACHINES_CONSENSUS_PATH: &str = "../dev-journal/artifacts/2026-08-28-pantheon-commons-machines-consensus.json";
const RHIZOME_PROVOCATION_PATH: &str = "../dev-journal/artifacts/2026-08-28-pantheon-commons-rhizome-provocation.json";

#[derive(Deserialize)]
struct ConsensusRun {
    final_sequence: Vec<String>,
}

#[derive(Deserialize)]
struct ProvocationEntry {
    respondent: String,
    subject: String,
    predicate: String,
    object: String,
}

#[derive(Debug, Clone)]
struct TurnRecord {
    cid: String,
    respondent: String,
    #[allow(dead_code)]
    true_author: String,
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
    true_author: &'a str,
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

// Tiny deterministic-but-varied shuffle (xorshift64 seeded from wall
// clock) so dispatch order changes every round without pulling in the
// `rand` crate as a new direct dependency of this crate.
struct Xorshift64(u64);
impl Xorshift64 {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

fn shuffled_order(rng: &mut Xorshift64) -> Vec<&'static Olympian> {
    let mut v: Vec<&'static Olympian> = OLYMPIANS.iter().collect();
    for i in (1..v.len()).rev() {
        let j = (rng.next() as usize) % (i + 1);
        v.swap(i, j);
    }
    v
}

fn dmml_turn_tool() -> ChatCompletionTools {
    ChatCompletionTools::Function(ChatCompletionTool {
        function: FunctionObject {
            name: "submit_dmml_turn".to_string(),
            description: Some("Submit exactly one DMML conversational turn.".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "verb": {"type": "string", "enum": ["argues", "questions", "extends", "disputes", "connects", "reflects", "ruptures", "elects", "hunts"]},
                    "subject": {"type": "string", "description": "short slug for what this turn is about"},
                    "predicate": {"type": "string", "description": "short camelCase predicate naming the claim"},
                    "object": {"type": "string", "description": "the actual claim, one to several sentences, in your own voice"},
                    "consumes": {
                        "type": "array",
                        "description": "at least one EXACT (cid, subject, predicate) copied from the log you were shown -- never invent one.",
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

async fn call_model(
    client: &Client<OpenAIConfig>,
    olympian: &Olympian,
    user_msg: String,
) -> anyhow::Result<DmmlTurnArgs> {
    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .reasoning_effort(ReasoningEffort::Low)
        .max_completion_tokens(1600u32)
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
                    uri: format!("iroh://pantheon-commons-constituent/{cid}"),
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
        true_author: String::new(),
        round,
        verb: verb.to_string(),
        subject: subject.to_string(),
        predicate: predicate.to_string(),
        object: object.to_string(),
        consumes: consumes_facts.to_vec(),
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChosenMode {
    Hunt,
    Sparagmos,
    Other,
}

fn detect_mode(elect_text: &str) -> ChosenMode {
    let lower = elect_text.to_lowercase();
    if lower.contains("sparagmos") || lower.contains("possession") || lower.contains("mask") {
        ChosenMode::Sparagmos
    } else if lower.contains("hunt") || lower.contains("pursu") || lower.contains("target") {
        ChosenMode::Hunt
    } else {
        ChosenMode::Other
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("== pantheon commons constituent: can the four choose their own rule? ==\n");

    let raw = std::fs::read_to_string(MACHINES_CONSENSUS_PATH)
        .map_err(|e| anyhow::anyhow!("couldn't read machines consensus at {MACHINES_CONSENSUS_PATH}: {e}"))?;
    let machines: ConsensusRun = serde_json::from_str(&raw)?;
    println!("loaded machines consensus ({} statements)", machines.final_sequence.len());

    let raw2 = std::fs::read_to_string(RHIZOME_PROVOCATION_PATH)
        .map_err(|e| anyhow::anyhow!("couldn't read rhizome provocation at {RHIZOME_PROVOCATION_PATH}: {e}"))?;
    let provocation_all: Vec<ProvocationEntry> = serde_json::from_str(&raw2)?;
    let protocols: Vec<&ProvocationEntry> = provocation_all
        .iter()
        .filter(|e| ["athena", "artemis", "apollo", "dionysus"].contains(&e.respondent.as_str()))
        .collect();
    println!("loaded {} rhizome-provocation protocol proposals\n", protocols.len());

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
    for src in ["machines", "protocols", "hardtnegri"] {
        source_authors.insert(src, api.author_create().await?);
    }
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        source_authors["machines"],
        "pantheon-commons-constituent".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} frozen machines-consensus items --", machines.final_sequence.len());
    let machines_author = source_authors["machines"];
    for (i, statement) in machines.final_sequence.iter().enumerate() {
        let subject = format!("machines_doc/item{i}");
        let mut rec = append(&substrate, &machines_author, 0, "statement", &subject, "statement", statement, &[]).await?;
        rec.respondent = "machines".to_string();
        rec.true_author = "machines".to_string();
        println!("  [item{i}] {} -> \"{}\"", rec.cid, rec.object);
        log.push(rec);
    }

    println!("\n-- seeding {} frozen rhizome-protocol proposals --", protocols.len());
    let protocols_author = source_authors["protocols"];
    for p in &protocols {
        let mut rec = append(&substrate, &protocols_author, 0, "proposes", &p.subject, &p.predicate, &p.object, &[]).await?;
        rec.respondent = format!("protocol_{}", p.respondent);
        rec.true_author = p.respondent.clone();
        println!("  [{}] {} -> \"{}\"", p.subject, rec.cid, &rec.object[..80.min(rec.object.len())]);
        log.push(rec);
    }

    println!("\n-- seeding {} new anchor claims (hardt+negri) --", NEW_ANCHORS.len());
    let hn_author = source_authors["hardtnegri"];
    for a in NEW_ANCHORS {
        let mut rec = append(&substrate, &hn_author, 0, a.verb, a.subject, a.predicate, a.object, &[]).await?;
        rec.respondent = "hardtnegri".to_string();
        rec.true_author = "hardtnegri".to_string();
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

    // ---- CHOICE ROUND: each Olympian picks/adapts a protocol for
    // THEMSELVES, individually, with no coordination or knowledge of
    // the others' choices in this same call.
    println!("\n-- choice round: which rule do you choose to follow? --");
    let mut modes: HashMap<&'static str, ChosenMode> = HashMap::new();
    for olympian in OLYMPIANS {
        print!("  dispatching {} (choosing)... ", olympian.name);
        use std::io::Write;
        std::io::stdout().flush().ok();
        let user_msg = format!(
            "Four proposals for how this pantheon's procedure could stop being arborescent were made in a \
prior round -- your own, and the other three Olympians'. They are seeded in the log below as \
respondent=protocol_athena, protocol_artemis, protocol_apollo, protocol_dionysus. Also seeded: Hardt and \
Negri's Multitude (respondent=hardtnegri), which argues the multitude is 'composed of singularities that \
act in common' -- not unified into one will, not incoherent either, but joined by what its members share \
without being reduced to sameness; and their Empire, which argues that a decentered, boundary-less network \
power can still be total, even without a single throne. This is a real, structural choice, not a thought \
experiment: nothing in DMML's own commit graph requires a fixed speaking order, numbered items, or a \
majority vote -- those were always this harness's own scaffolding, never load-bearing. You may now actually \
choose, for the debate about to happen, which of the four protocols governs YOUR OWN turns -- not the whole \
group's, just yours. You do not know what the other three will choose, and you should not try to coordinate \
-- choose based on what you actually believe is right, the way Hardt and Negri's multitude is supposed to \
cohere without a sovereign deciding for everyone. You may adopt one protocol as your own (Athena's web, \
Artemis's hunt, Apollo's lyre, or Dionysus's sparagmos), hybridize two, reject all four and propose your \
own, or elect to keep following the old fixed-order status quo if you genuinely believe that is more honest \
than performing rhizome. Say which, and why, concretely enough that it will be possible to tell, turn by \
turn, whether you are actually following it.\n\n\
Log:\n{}\n\n\
Use verb `elects`. `consumes` must include at least one real (cid, subject, predicate) from the log above, \
ideally the protocol you are adopting or reacting against.",
            transcript_so_far(&log)
        );
        match call_model(&client, olympian, user_msg).await {
            Ok(reply) => {
                let mode = detect_mode(&reply.object);
                modes.insert(olympian.name, mode);
                let mut verified = Vec::new();
                for c in &reply.consumes {
                    let real = log.iter().any(|t| t.cid == c.cid && t.subject == c.subject && t.predicate == c.predicate);
                    if real {
                        verified.push((c.cid.clone(), c.subject.clone(), c.predicate.clone()));
                    }
                }
                let author = olympian_authors[olympian.name];
                let mut rec = append(&substrate, &author, 0, &reply.verb, &reply.subject, &reply.predicate, &reply.object, &verified).await?;
                rec.respondent = olympian.name.to_string();
                rec.true_author = olympian.name.to_string();
                let mode_str = match mode {
                    ChosenMode::Hunt => "HUNT",
                    ChosenMode::Sparagmos => "SPARAGMOS",
                    ChosenMode::Other => "other/web/lyre/status-quo",
                };
                println!("ok [{mode_str}] -> {} : \"{}\"", rec.cid, &rec.object[..120.min(rec.object.len())]);
                log.push(rec);
            }
            Err(e) => {
                println!("FAILED: {e}");
                modes.insert(olympian.name, ChosenMode::Other);
            }
        }
    }
    println!();
    for o in OLYMPIANS {
        let m = modes.get(o.name).copied().unwrap_or(ChosenMode::Other);
        let m_str = match m {
            ChosenMode::Hunt => "hunt (will name a citation target each turn)",
            ChosenMode::Sparagmos => "sparagmos (attribution reassigned by lot each turn)",
            ChosenMode::Other => "web/lyre/status-quo (no mechanical override beyond reshuffled order)",
        };
        println!("  {} chose: {m_str}", o.name);
    }

    let mut rng = Xorshift64(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64 | 1,
    );

    // ---- DEBATE ROUNDS: order reshuffled every round; hunt-mode
    // agents get a target-naming instruction; sparagmos-mode agents
    // have their public attribution reassigned by lot after drafting.
    for round in 1..=ROUNDS {
        let order = shuffled_order(&mut rng);
        println!("\n-- round {round} (order: {}) --", order.iter().map(|o| o.name).collect::<Vec<_>>().join(", "));
        for olympian in order {
            print!("  dispatching {}... ", olympian.name);
            use std::io::Write;
            std::io::stdout().flush().ok();

            let mode = modes.get(olympian.name).copied().unwrap_or(ChosenMode::Other);
            let mode_instruction = match mode {
                ChosenMode::Hunt => "\n\nYou elected Artemis's hunt protocol for yourself: before making your move, name the specific existing claim (cid, subject) in the log you are hunting this turn, and frame your move as pursuit of it, not commentary from a fixed seat.",
                ChosenMode::Sparagmos => "\n\nYou elected Dionysus's sparagmos protocol for yourself: write your turn as you genuinely think it, in your own real voice -- your public attribution will be reassigned by lot after you submit it, so do not perform someone else's voice; that reassignment is not yours to control.",
                ChosenMode::Other => "",
            };

            let user_msg = format!(
                "Continue the constituent-power debate: does letting each Olympian choose their own \
governing rule (rather than one rule imposed on all four by a vote) produce Hardt and Negri's multitude -- \
plural, coherent through what is held in common, without being reduced to one will -- or does it just \
reproduce Empire's own decentered, boundary-less, still-total capture at the pantheon's own scale? Test \
this against the machines_doc items, the four protocol proposals, and the hardtnegri anchors. Does a \
specific claim get CONFIRMED, EXTENDED, or RUPTURED? Note: dispatch order this round was reshuffled \
({order_names}) -- there is no fixed seat.{mode_instruction}\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona. `consumes` must copy at least \
one real (cid, subject, predicate) from the log above exactly -- never invent one. Use verb `ruptures` if \
breaking a claim, `hunts` if this is specifically a hunt-protocol pursuit move, or `argues`/`extends`/\
`disputes`/`connects` otherwise. Call submit_dmml_turn with your answer.",
                transcript_so_far(&log),
                order_names = OLYMPIANS.iter().map(|o| o.name).collect::<Vec<_>>().join(", "),
            );

            match call_model(&client, olympian, user_msg).await {
                Ok(reply) => {
                    let mut verified = Vec::new();
                    for c in &reply.consumes {
                        let real = log.iter().any(|t| t.cid == c.cid && t.subject == c.subject && t.predicate == c.predicate);
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
                    let mut rec = append(&substrate, &author, round as u32, &reply.verb, &reply.subject, &reply.predicate, &reply.object, &verified).await?;
                    rec.true_author = olympian.name.to_string();

                    if mode == ChosenMode::Sparagmos {
                        let others: Vec<&'static str> = OLYMPIANS.iter().map(|o| o.name).filter(|n| *n != olympian.name).collect();
                        let pick = (rng.next() as usize) % others.len();
                        rec.respondent = others[pick].to_string();
                        println!(
                            "ok -> {} : {} {} \"{}\" (consumes {}) [MASK: born {}, worn as {}]",
                            rec.cid, rec.subject, rec.predicate, rec.object, verified.len(), olympian.name, rec.respondent
                        );
                    } else {
                        rec.respondent = olympian.name.to_string();
                        println!(
                            "ok -> {} : {} {} \"{}\" (consumes {})",
                            rec.cid, rec.subject, rec.predicate, rec.object, verified.len()
                        );
                    }
                    log.push(rec);
                }
                Err(e) => println!("FAILED: {e}"),
            }
        }
    }

    // ---- No formal ratification vote. "Kept alive by use": a claim
    // from rounds 1..=ROUNDS is ratified-by-use if it was cited at
    // least twice, by at least two DISTINCT true authors, anywhere
    // later in the log.
    println!("\n-- ratified-by-use tally (no vote; Athena's own proposed mechanism, applied) --");
    let debate_turns: Vec<&TurnRecord> = log.iter().filter(|t| t.round > 0).collect();
    let mut citation_counts: HashMap<String, (u32, std::collections::HashSet<String>)> = HashMap::new();
    for t in &log {
        for (cid, _, _) in &t.consumes {
            let entry = citation_counts.entry(cid.clone()).or_insert((0, std::collections::HashSet::new()));
            entry.0 += 1;
            entry.1.insert(t.true_author.clone());
        }
    }
    let mut ratified: Vec<&TurnRecord> = Vec::new();
    for t in &debate_turns {
        if let Some((count, authors)) = citation_counts.get(&t.cid) {
            if *count >= 2 && authors.len() >= 2 {
                ratified.push(t);
                println!("  RATIFIED (cited {count}x by {} distinct authors): {} -- \"{}\"", authors.len(), t.cid, &t.object[..100.min(t.object.len())]);
            }
        }
    }
    println!("\n{} of {} debate turns kept alive by use.", ratified.len(), debate_turns.len());

    println!("\n-- final transcript: {} real entries --", log.len());
    let dumped: Vec<DumpedTurn> = log
        .iter()
        .map(|t| DumpedTurn {
            cid: &t.cid,
            respondent: &t.respondent,
            true_author: &t.true_author,
            round: t.round,
            verb: &t.verb,
            subject: &t.subject,
            predicate: &t.predicate,
            object: &t.object,
            consumes: &t.consumes,
        })
        .collect();
    let json = serde_json::to_string_pretty(&dumped)?;
    std::fs::write("pantheon_commons_constituent.json", &json)?;
    println!("wrote pantheon_commons_constituent.json ({} entries)", dumped.len());

    Ok(())
}
