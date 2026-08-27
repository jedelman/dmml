//! Phase 4: the remaining embodiment/shamanism-adjacent thinkers in
//! Power Explained's dramatis-personae roster, per Jason's follow-up:
//! "Eliade, Taussig, Ginzburg, and Kopenwa for next round." All four are
//! real `G-DP-*` profiles there (Eliade `G-DP-020`, Taussig `G-DP-021`,
//! Ginzburg `G-DP-022`, Kopenawa `G-DP-023`), and all four bear directly
//! on the embodiment run's own unresolved seat-at-the-table question:
//! Eliade's shamanism (archaic, learnable techniques of ecstasy, not a
//! private mystical claim); Taussig's mimesis and alterity (the copy's
//! power over what it represents, and the colonizer's own mimicry of
//! what he feared); Ginzburg's benandanti (real people whose own
//! self-understanding was reshaped by a persecuting institution's
//! template over a century, and a method for reading history's own
//! excluded testimony against the record that suppressed it); Kopenawa's
//! xapiri and "the people of merchandise" (a living, non-Western
//! account of embodied ritual practice keeping a real cosmological
//! order intact).
//!
//! Structurally: same frozen-prior-plus-new-anchors pattern as
//! `pantheon_commons_rupture.rs` and `pantheon_commons_embodiment.rs`,
//! extending the EMBODIMENT run's own consensus (`pantheon_commons_
//! embodiment_consensus.rs`'s output) -- which explicitly concluded that
//! no criterion for the reconciliation's open seat survived, and that
//! the demand for one was itself the armor under study. `DispatchMode::
//! Extend` invites confirmation, extension, or rupture of specific
//! embodiment-consensus items using this round's four new sources.

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

// The 32 new anchors. Every claim is either a verified direct quote or
// an explicitly labeled paraphrase of a well-documented argument, same
// discipline as every anchor in this project.
const NEW_ANCHORS: &[Anchor] = &[
    // Mircea Eliade, The Sacred and the Profane (1957) and Shamanism (1951).
    Anchor { id: "eliade/hierophany", author: "eliade", verb: "argues", subject: "sacred_profane/hierophany", predicate: "claim", object: "A hierophany is a manifestation of the sacred that breaks through the homogeneous, undifferentiated expanse of profane space and reveals an absolute fixed point, a center, around which orientation becomes possible" },
    Anchor { id: "eliade/sacred_profane_division", author: "eliade", verb: "argues", subject: "sacred_profane/sacred_profane_division", predicate: "claim", object: "Religion for archaic and traditional peoples rests on a sharp division between the sacred and the profane, two fundamentally different modes of being in the world" },
    Anchor { id: "eliade/axis_mundi", author: "eliade", verb: "argues", subject: "sacred_profane/axis_mundi", predicate: "claim", object: "The axis mundi -- the cosmic tree, pillar, or mountain -- names the point of contact between heaven, earth, and underworld established by an initial hierophany, a structural feature repeated across unrelated traditions" },
    Anchor { id: "eliade/myth_as_breakthrough", author: "eliade", verb: "argues", subject: "sacred_profane/myth_as_breakthrough", predicate: "claim", object: "Myths describe breakthroughs of the sacred into the world -- not fictions to be explained away but the record of a real, structuring encounter for the people who tell them" },
    Anchor { id: "eliade/shaman_archaic_technique", author: "eliade", verb: "argues", subject: "shamanism/archaic_technique_of_ecstasy", predicate: "claim", object: "Shamanism names a comparative, cross-cultural set of archaic techniques of ecstasy -- real, learnable practices of trance, soul-flight, and ritual death and rebirth for crossing between the sacred and profane, not a single culture's peculiar belief" },
    Anchor { id: "eliade/eternal_return", author: "eliade", verb: "argues", subject: "sacred_profane/eternal_return", predicate: "claim", object: "Archaic societies periodically abolish historical time through ritual, returning symbolically to a mythical moment of origin -- the eternal return -- rather than experiencing time as linear and irreversible" },
    Anchor { id: "eliade/terror_of_history", author: "eliade", verb: "disputes", subject: "sacred_profane/terror_of_history", predicate: "claim", object: "Modern historical consciousness, cut off from the eternal return, produces the terror of history -- suffering experienced as meaningless because it cannot be redeemed by any cyclical, sacred framework" },
    Anchor { id: "eliade/initiatory_death", author: "eliade", verb: "argues", subject: "shamanism/initiatory_death", predicate: "claim", object: "Initiation rites across unrelated cultures share a common structure of symbolic death and rebirth, marking the novice's passage from a profane to a sacred mode of existence" },
    // Michael Taussig, Mimesis and Alterity (1993).
    Anchor { id: "taussig/mimetic_faculty", author: "taussig", verb: "argues", subject: "mimesis_alterity/mimetic_faculty", predicate: "claim", object: "The mimetic faculty -- the compulsion to copy, imitate, and become Other -- is a real, historically documented human capacity, not merely a metaphor, and colonial contact put it under specific new pressure" },
    Anchor { id: "taussig/colonial_wildman_mimesis", author: "taussig", verb: "argues", subject: "mimesis_alterity/colonial_wildman_mimesis", predicate: "claim", object: "The history of mimesis is bound up with Euroamerican colonialism's own fantasy of the primitive's supposed mimetic prowess, a fascination the colonizer projected onto the colonized while denying possessing the same faculty himself" },
    Anchor { id: "taussig/copy_affects_original", author: "taussig", verb: "argues", subject: "mimesis_alterity/copy_affects_original", predicate: "claim", object: "Sympathetic magic's old principle -- that the copy affects the original, that a likeness can act on what it resembles -- persists as a real structuring logic in modern technological society, not just in primitive ritual" },
    Anchor { id: "taussig/colonial_mirror", author: "taussig", verb: "disputes", subject: "mimesis_alterity/colonial_mirror", predicate: "claim", object: "Colonial violence generated its own mimicry of the violence it feared from its subjects, a mirroring in which the colonizer became what he claimed only the savage could be" },
    Anchor { id: "taussig/wildman_projection", author: "taussig", verb: "disputes", subject: "mimesis_alterity/wildman_projection", predicate: "claim", object: "The figure of the Wild Man is less a discovery about the colonized than a mirror held up to the colonizer's own repressed and projected fears and desires" },
    Anchor { id: "taussig/mimesis_and_alterity_double", author: "taussig", verb: "argues", subject: "mimesis_alterity/double_sided_mimesis", predicate: "claim", object: "Mimesis is always double-sided: the copy is never neutral reproduction but a way of registering, and sometimes seizing power over, the alterity it represents" },
    Anchor { id: "taussig/senses_history", author: "taussig", verb: "argues", subject: "mimesis_alterity/senses_history", predicate: "claim", object: "The history of the senses -- touch, contact, tactility -- is itself a real historical and political terrain, not a neutral biological substrate underneath history" },
    Anchor { id: "taussig/camera_as_mimetic_machine", author: "taussig", verb: "argues", subject: "mimesis_alterity/camera_as_mimetic_machine", predicate: "claim", object: "Nineteenth-century mimetic technologies like the camera intensify, rather than replace, the older sympathetic-magical logic of the copy acting on the original" },
    // Carlo Ginzburg, The Night Battles (1966) and Ecstasies (1989).
    Anchor { id: "ginzburg/benandanti_good_walkers", author: "ginzburg", verb: "asserts", subject: "night_battles/benandanti_good_walkers", predicate: "claim", object: "The benandanti, the good walkers, of 16th-century Friuli believed their souls left their sleeping bodies on the four Ember nights of the year to do real, ritual battle with witches over the harvest" },
    Anchor { id: "ginzburg/benandanti_fought_for_harvest", author: "ginzburg", verb: "argues", subject: "night_battles/benandanti_fought_for_harvest", predicate: "claim", object: "The benandanti understood themselves as defenders of the community's crops, armed with fennel stalks against witches armed with sorghum -- an agrarian fertility cult, not devil worship, in its own original self-understanding" },
    Anchor { id: "ginzburg/inquisition_reshapes_benandanti", author: "ginzburg", verb: "disputes", subject: "night_battles/inquisition_reshapes_benandanti", predicate: "claim", object: "Over a century of interrogation, the Holy Office progressively pressured the benandanti's own testimony to fit the pre-existing template of the witches' sabbath, until the benandanti's self-description began to change to match it" },
    Anchor { id: "ginzburg/microhistory_method", author: "ginzburg", verb: "argues", subject: "night_battles/microhistory_method", predicate: "claim", object: "Ginzburg's method reconstructs the actual beliefs of ordinary, non-literate people from the distorting filter of inquisitorial records, reading against the interrogators' own assumptions to recover a real, different cosmology underneath" },
    Anchor { id: "ginzburg/witches_sabbath_substrate", author: "ginzburg", verb: "argues", subject: "ecstasies/witches_sabbath_substrate", predicate: "claim", object: "The imagery of the witches' sabbath -- night flight, animal metamorphosis, ecstatic gathering -- has a real, deep substrate in a much older, widespread Eurasian shamanistic complex, not merely inquisitorial fantasy" },
    Anchor { id: "ginzburg/circumpolar_connections", author: "ginzburg", verb: "argues", subject: "ecstasies/circumpolar_connections", predicate: "claim", object: "Ecstasies traces structural parallels between the European witches' sabbath and circumpolar and Eurasian shamanic traditions across enormous geographic and temporal distance -- a real, if disputed, morphological connection" },
    Anchor { id: "ginzburg/benandanti_became_witches", author: "ginzburg", verb: "asserts", subject: "night_battles/benandanti_became_witches", predicate: "claim", object: "The slow metamorphosis of the benandanti into the very witches they claimed to fight is a real, documented case of a persecuting institution actually reshaping the self-understanding and practice of the persecuted over time" },
    Anchor { id: "ginzburg/clues_paradigm", author: "ginzburg", verb: "argues", subject: "night_battles/clues_paradigm", predicate: "claim", object: "Ginzburg's clues (evidential) paradigm argues history can be read like a hunter reads tracks -- from small, overlooked details -- recovering what dominant narratives suppress or never recorded directly" },
    // Davi Kopenawa (with Bruce Albert), The Falling Sky (2013).
    Anchor { id: "kopenawa/xapiri_spirits", author: "kopenawa", verb: "asserts", subject: "falling_sky/xapiri_spirits", predicate: "claim", object: "The xapiri are real spirit-images of the forest's animal ancestors that a trained shaman sees, hosts, and works with directly -- not metaphor or belief but Kopenawa's own reported direct experience and vocation" },
    Anchor { id: "kopenawa/white_people_dont_see", author: "kopenawa", verb: "disputes", subject: "falling_sky/white_people_dont_see", predicate: "claim", object: "White people do not see the xapiri and have lost the ancestors' ways, which Kopenawa presents as a real perceptual and spiritual deficit of the colonizing society, not a neutral difference in belief" },
    Anchor { id: "kopenawa/people_of_merchandise", author: "kopenawa", verb: "disputes", subject: "falling_sky/people_of_merchandise", predicate: "claim", object: "Kopenawa names those driven by economic greed the people of merchandise, arguing this replaces spiritual ecology and love of the forest with an arrogant love of commodities that threatens the forest's survival" },
    Anchor { id: "kopenawa/falling_sky_warning", author: "kopenawa", verb: "asserts", subject: "falling_sky/falling_sky_warning", predicate: "claim", object: "The sky itself is held up by the xapiri's constant ritual work, and the book's title names a real cosmological warning: if the shamans' work and the forest that sustains it are destroyed, the sky can fall" },
    Anchor { id: "kopenawa/testimony_as_political_act", author: "kopenawa", verb: "argues", subject: "falling_sky/testimony_as_political_act", predicate: "claim", object: "Kopenawa's whole narrated testimony is presented as a direct political act -- a shaman speaking for the forest and his people to a world of readers who will likely never visit it, not an ethnographic object collected about him" },
    Anchor { id: "kopenawa/gold_miners_disease", author: "kopenawa", verb: "argues", subject: "falling_sky/gold_miners_disease", predicate: "claim", object: "The arrival of gold miners on Yanomami land brought real, documented epidemic disease and ecological destruction, narrated as a direct consequence of the people of merchandise's greed" },
    Anchor { id: "kopenawa/shamanic_knowledge_transmission", author: "kopenawa", verb: "argues", subject: "falling_sky/shamanic_knowledge_transmission", predicate: "claim", object: "Shamanic knowledge is transmitted through a real, disciplined apprenticeship -- fasting, isolation, ingesting yakoana snuff, learning from senior shamans -- a rigorous practice, not a spontaneous gift" },
    Anchor { id: "kopenawa/forest_as_living_entity", author: "kopenawa", verb: "disputes", subject: "falling_sky/forest_as_living_entity", predicate: "claim", object: "The forest itself is a living entity with its own spirit-inhabitants and agency, not a passive resource -- a real ontological claim the testimony insists readers take seriously rather than read as folklore" },
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
const EMBODIMENT_CONSENSUS_PATH: &str = "../dev-journal/artifacts/2026-08-27-pantheon-commons-embodiment-consensus.json";

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
                        "description": "at least one EXACT (cid, subject, predicate) copied from the log you were shown -- never invent one. Prefer citing at least one embodiment_doc item AND one new anchor when making a claim, to keep the citation concrete.",
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
            "Four Olympians (Athena, Artemis, Apollo, Dionysus) produced an EMBODIMENT CONSENSUS \
document (respondent=embodiment, entries subject embodiment_doc/item0 through item8) after a debate \
that tested a real reconciliation document against Wilhelm Reich (character armor, repression braced \
into muscle) and Bill Plotkin (the descent to soul). That debate proposed and broke five successive \
criteria for who may hold an open seat at the rule-making table -- certified descent, an unadministered \
vision fast, a grammar of deeds, ecstasy itself, a retroactive trace-test -- and concluded that the \
demand for an incorruptible criterion was itself a symptom of the armor under study. That consensus is \
real and checkpointed, but its own item9 refuses to convert this finding into a new qualification and \
leaves the underlying question open. You are not re-arguing the whole thing from scratch; you are \
extending it with four new real sources: Mircea Eliade's Shamanism and The Sacred and the Profane \
(respondent=eliade), on shamanism as a comparative set of archaic, LEARNABLE techniques of ecstasy, not \
a private mystical gift, and on hierophany as a real structuring encounter; Michael Taussig's Mimesis \
and Alterity (respondent=taussig), on the mimetic faculty and how the colonizer's own fantasy of the \
primitive's mimetic prowess was a mirror of his own repressed capacities; Carlo Ginzburg's The Night \
Battles and Ecstasies (respondent=ginzburg), on the benandanti -- real people whose self-understanding \
was reshaped over a century by the very inquisition that persecuted them, and on his method for reading \
history's suppressed testimony against the record; and Davi Kopenawa's The Falling Sky (respondent= \
kopenawa), a living, non-Western testimony describing shamanic apprenticeship as a real, rigorous, \
transmissible discipline (not a spontaneous gift) sustaining an actual cosmological order. Test the new \
material against the embodiment consensus, specifically: does Eliade's claim that shamanic ecstasy is a \
LEARNABLE technique undercut the embodiment debate's own conclusion that no criterion for the seat could \
be taught or certified? Does Ginzburg's benandanti -- real people whose testimony was progressively \
reshaped by their persecutors -- offer a real historical case of exactly the 'certifier rebuilds the \
gate' failure the embodiment debate diagnosed abstractly? Does Kopenawa's actual, disciplined shamanic \
apprenticeship function as a working counterexample to 'no criterion survives,' or does it fail for the \
same reasons the embodiment debate's proposals failed? Does a specific numbered item in embodiment_doc \
get CONFIRMED, EXTENDED into territory it didn't cover, or actually RUPTURED? Other entries in the log \
are prior turns from you or the other three Olympians in THIS debate.\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona. Prefer citing at least one \
real embodiment_doc item together with at least one real new anchor from Eliade, Taussig, Ginzburg, or \
Kopenawa, so the move is concrete and checkable, not a vague gesture. If you don't have a real move to \
make, say so honestly in `object` rather than padding. Use verb `ruptures` if you are specifically \
breaking or overturning an embodiment_doc item; use `argues`, `extends`, `disputes`, or `connects` for \
other moves. Call submit_dmml_turn with your answer. `consumes` must copy at least one real (cid, \
subject, predicate) from the log above exactly -- never invent one.",
            transcript_so_far(log)
        ),
        DispatchMode::Reflect => format!(
            "The debate among the four Olympians -- testing Eliade, Taussig, Ginzburg, and Kopenawa's \
shamanism/alterity material against the embodiment consensus's items -- has just ended. Here is the \
complete real transcript, everything anyone actually said, in order:\n\n{}\n\nThe debate is over -- this \
is not another argumentative move. Reflect, in your own voice as this persona, on your OWN trajectory \
through it: did encountering these four sources actually change how you'd now describe the embodiment \
consensus's conclusion that no criterion for the seat survived -- do you now think a real, working \
criterion exists after all (certified apprenticeship, say), or does the new material just confirm the \
prior finding from a different angle? Name what specifically moved you (a turn, a phrase, a specific new \
anchor), if anything did, or say honestly that nothing did and why not -- false movement is as dishonest \
as false stillness. If you can, cite your own earliest turn in this debate and something later that \
responds to or revises it. Use verb `reflects`. `consumes` should include your own earlier turn if you \
can find one, exactly as it appears in the log -- never invent a citation.",
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
                    uri: format!("iroh://pantheon-commons-shamanism/{cid}"),
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
    println!("== pantheon commons shamanism: Eliade+Taussig+Ginzburg+Kopenawa vs. the open embodiment consensus ==\n");

    let raw = std::fs::read_to_string(EMBODIMENT_CONSENSUS_PATH)
        .map_err(|e| anyhow::anyhow!("couldn't read embodiment consensus at {EMBODIMENT_CONSENSUS_PATH}: {e}"))?;
    let phase1: ConsensusRun = serde_json::from_str(&raw)?;
    println!("loaded reconciliation document ({} statements)\n", phase1.final_sequence.len());

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
    for src in ["embodiment", "eliade", "taussig", "ginzburg", "kopenawa"] {
        source_authors.insert(src, api.author_create().await?);
    }
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        source_authors["reconciliation"],
        "pantheon-commons-shamanism".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} frozen reconciliation-document items --", phase1.final_sequence.len());
    // verb == predicate == "statement" here, deliberately: the rupture run
    // (pantheon_commons_rupture.rs) found models repeatedly citing the verb
    // field ("ratifiedAs") as if it were the predicate field ("statement"),
    // a real, root-caused schema ambiguity from using two different strings
    // for those fields on frozen prior-consensus items -- see dev-journal
    // 2026-08-27-pantheon-commons-rupture.md. Using the same string for
    // both fields here makes that specific confusion harmless.
    let synthesis_author = source_authors["reconciliation"];
    for (i, statement) in phase1.final_sequence.iter().enumerate() {
        let subject = format!("embodiment_doc/item{i}");
        let mut rec = append(&substrate, &synthesis_author, 0, "statement", &subject, "statement", statement, &[]).await?;
        rec.respondent = "reconciliation".to_string();
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
    std::fs::write("pantheon_commons_shamanism.json", &json)?;
    println!("wrote pantheon_commons_shamanism.json ({} entries)", dumped.len());

    Ok(())
}
