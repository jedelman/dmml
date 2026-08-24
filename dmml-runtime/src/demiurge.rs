//! The demiurge: now bootstrap-only. Filling a frontier point used to be
//! this module's own singular, uniform act -- one fixed noun table, no
//! notion that different territory could want a different hand. That's the
//! "monotony of symmetric forms" jedelman/written-world#8 names: the fix
//! isn't a bigger table, it's admitting more than one generative agency.
//!
//! **The pantheon.** A Theos (Pluto: stone and metal; Demeter: vines and
//! roots breaking ruins) is ordinary content, not privileged code -- ground
//! by the identical `Effect::GenerateFrontier` machinery any future
//! generative agency (Saturn, Mercury, a petition-sourced one) would use,
//! equipped via `build_action_machine` the same way a lever's drift
//! mechanism is. Which Theos governs a given frontier point is read off
//! whichever generator machine is equipped to the *origin* room
//! (`theos_generator_domain`); a room with none yet gets one rolled and
//! equipped on first expansion (`ensure_theos_generator`), and the newly
//! generated room inherits the same domain, so territory reads as
//! *regions*, not per-room noise. Saturn (time) and Mercury
//! (causality-tracing) aren't built yet -- neither has an obvious noun-pool
//! shape the way a material does, and inventing their mechanics without
//! more design discussion would just relocate the "monotony" problem
//! instead of fixing it. Two real Theoi, proving the mechanism, beats four
//! speculative ones.
//!
//! Bootstrapping the player's initial machines still goes through exactly
//! the same `WorldGraph::commit` gate as anything a player, an NPC, or a
//! Theos proposes. It's still invoked procedurally from fixed moments
//! (`Game::new`, crossing a frontier edge) rather than running its own
//! autonomous desire-loop — see the session notes on why that's an honest
//! scope cut, not a principled one: the mechanism generalizes, this
//! prototype just doesn't drive it from a general scheduler yet.

use oxigraph::model::{NamedNode, Quad, Term};

use crate::character;
use crate::direction::Direction;
use crate::graph::{
    as_node, as_string, lit_bool, lit_float, lit_int, lit_str, Commit, Delta, StrongRef,
    WorldGraph,
};
use crate::machine::{self, build_action_machine, build_sense_machine, Effect, Requirement};
use crate::rng::Rng;
use crate::vocab;

pub const LEVER_PULL_STEP: f32 = 0.34;
pub const LEVER_WEAR_THRESHOLD: f32 = 1.0;

// Rooms no longer get an independently-rolled adjective or detail
// sentence -- ROOM_NOUN is the one purely categorical (not fact-derived)
// choice left, since "hallway" vs "cellar" is architectural, not mood.
// The adjective baked into a room's name, and its whole description, both
// come from the same rolled dampness/decay/light facts via character.rs.
const ROOM_NOUN: &[&str] = &[
    "chamber",
    "hallway",
    "cellar",
    "landing",
    "vault",
    "gallery",
    "cistern",
    "alcove",
    "stairwell",
    "nook",
    "antechamber",
    "passage",
];
const ITEM_NOUN: &[&str] = &[
    "cracked mirror",
    "clay jug",
    "moth-eaten cloak",
    "tarnished locket",
    "bundle of dry reeds",
    "chipped bowl",
    "coil of rope",
    "brass whistle",
    "warped hand mirror",
    "handful of bone dice",
];

/// Registered Theoi -- the pantheon's actual membership. `theosDomain` is
/// one of these string identifiers, never validated against this list by
/// the schema itself (that would make the pantheon closed vocabulary,
/// which it isn't -- a future petition or content pass can propose a
/// domain this list doesn't yet know about, same as any self-declared
/// relation), but this is what `ensure_theos_generator` rolls among today.
const THEOI: &[&str] = &["pluto", "demeter"];

const PLUTO_ROOM_NOUN: &[&str] = &[
    "quarry",
    "shaft",
    "ore chamber",
    "smeltworks",
    "gallery",
    "cistern of slag",
    "vault",
    "stope",
    "foundry floor",
    "cavern",
];
const PLUTO_ITEM_NOUN: &[&str] = &[
    "pitted iron ingot",
    "shard of raw ore",
    "corroded chisel",
    "lump of native copper",
    "cracked grindstone",
    "tarnished coin",
    "bent crowbar",
    "chunk of slag glass",
    "rusted manacle",
    "handful of gravel",
];

const DEMETER_ROOM_NOUN: &[&str] = &[
    "overgrown hall",
    "root cellar",
    "vine-choked passage",
    "mossy landing",
    "collapsed greenhouse",
    "bramble thicket",
    "flooded grove",
    "orchard ruin",
    "canopy hollow",
    "creeper-bound stairwell",
];
const DEMETER_ITEM_NOUN: &[&str] = &[
    "knotted root",
    "handful of dry seed pods",
    "moss-covered stone",
    "coil of ivy",
    "withered garland",
    "cracked terracotta pot",
    "bundle of thorned cane",
    "rotted trellis slat",
    "cluster of fungus",
    "husk of a gourd",
];

fn room_nouns_for(domain: &str) -> &'static [&'static str] {
    match domain {
        "pluto" => PLUTO_ROOM_NOUN,
        "demeter" => DEMETER_ROOM_NOUN,
        _ => ROOM_NOUN,
    }
}

fn item_nouns_for(domain: &str) -> &'static [&'static str] {
    match domain {
        "pluto" => PLUTO_ITEM_NOUN,
        "demeter" => DEMETER_ITEM_NOUN,
        _ => ITEM_NOUN,
    }
}

/// Seeded off `(world_seed, origin)` only, deliberately excluding
/// direction -- a room's governing Theos doesn't depend on which way it's
/// first approached from, unlike a frontier point's own content
/// (`seed_for`, below, which does hash direction in).
fn theos_seed_for(world_seed: u64, origin: &NamedNode) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    world_seed.hash(&mut h);
    origin.as_str().hash(&mut h);
    "theos".hash(&mut h);
    h.finish()
}

/// Builds a `GenerateFrontier` machine equipped to `owner` -- a Theos
/// claiming generative dominion, same shape a lever's drift mechanism is
/// built with, just a different `Effect` kind.
fn build_theos_generator(graph: &mut WorldGraph, owner: &NamedNode, domain: &str) -> (NamedNode, Delta) {
    build_action_machine(
        graph,
        owner,
        "generate",
        &[],
        &Effect::GenerateFrontier {
            domain: domain.to_string(),
        },
    )
}

/// Which Theos already governs frontier generation from `origin`, if any
/// -- reads the first `generate`-triggered machine equipped there back
/// through the ordinary `Effect` infrastructure.
fn theos_generator_domain(graph: &WorldGraph, origin: &NamedNode) -> Option<String> {
    machine::machines_for_verb(graph, origin, "generate")
        .into_iter()
        .find_map(|m| {
            let effect_node = as_node(graph.object(&m, &vocab::has_effect())?)?;
            match machine::read_effect(graph, &effect_node)? {
                Effect::GenerateFrontier { domain } => Some(domain),
                _ => None,
            }
        })
}

/// The domain governing frontier generation from `origin` -- its existing
/// generator if one's already equipped, or a freshly rolled one (equipped
/// on the spot, so every later expansion from the same room agrees). This
/// is how a Theos's territory comes to cover more than a single room
/// without anything pre-partitioning the map in advance: dominion spreads
/// outward from wherever a player first pushes past a room's own frontier,
/// one expansion at a time (see `generate_frontier`'s own propagation onto
/// the room it mints).
fn ensure_theos_generator(graph: &mut WorldGraph, world_seed: u64, origin: &NamedNode) -> String {
    if let Some(domain) = theos_generator_domain(graph, origin) {
        return domain;
    }
    let mut rng = Rng::new(theos_seed_for(world_seed, origin));
    let domain = (*rng.pick(THEOI)).to_string();
    let (_, d) = build_theos_generator(graph, origin, &domain);
    graph
        .commit("demiurge", d)
        .expect("theos generator machine is always valid");
    domain
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// `seed` is the world's own immutable lineage root (`vocab::class_seed`'s
/// own doc comment) -- the concrete answer to "world-gen content needs
/// commit lineage" (#50 Tier 1 item 2's actual blocker): everything
/// `bootstrap` and `generate_frontier` mint from here on can trace back to
/// it via `Commit::via`, giving world-gen content the same real,
/// addressable commit lineage a player's own `take`/`drop`/`go` already
/// has -- unlike before, when genesis (and everything grown from it) had
/// none at all.
pub struct Bootstrap {
    pub player: NamedNode,
    pub start_room: NamedNode,
    pub seed: NamedNode,
}

/// The demiurge's inaugural act: mint the world's own seed, lay down the
/// Threshold, place the player in it, and equip the player's baseline
/// machines. This is the answer to "where does the sensing machine come
/// from" -- from here, chronologically first, not exempt from validation,
/// just prior to everything else.
pub fn bootstrap(graph: &mut WorldGraph) -> Bootstrap {
    // The world's own immutable lineage root -- minted first, via
    // `apply_commit` as a pure mint (`consumes: []`), so it's a real,
    // durably-addressed (`vocab::foreign_uri_node`) node everything else
    // in this function can point back to via `Commit::via`. Nothing ever
    // consumes it: there's nothing to retract about a world's own origin.
    // Also self-declares `locatedIn`/`heldBy` as Relations here, moved
    // from the old genesis `Delta` below -- still needed for `Game::
    // replay_commit`'s validated re-run of every transcript entry (see
    // those predicates' own doc comments), just asserted via this commit
    // now instead of a separate one.
    let seed_raw = graph.fresh("seed/");
    let seed = vocab::foreign_uri_node(seed_raw.as_str());
    let seed_quads = vec![
        Quad::new(
            seed.clone(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_seed()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::located_in(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_relation()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::held_by(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_relation()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        // Self-declares the DMML petition state machine's own predicates
        // (`commune::raise_petition_dmml`/`reply_petition_dmml`/
        // `accept_petition_dmml`) -- see `vocab.rs`'s "DMML petition state
        // machine" doc comment for why this is a second, additive
        // mechanism, not a migration of the existing one.
        Quad::new(
            vocab::dmml_petition_status(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_relation()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::replies_to(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_relation()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::petition_reply_content(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_attribute()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        // Self-declares the eight `machine.rs`-anatomy predicates
        // (`trigger`/`requirementKind`/`effectKind`/`effectDomain`/
        // `requirementLockedValue`/`effectLockedValue`/`effectStep`/
        // `requirementThreshold`) that used to be exempted from
        // self-declaration by dedicated name in `graph::
        // is_closed_vocabulary` -- same generic self-declared-Attribute
        // mechanism `locatedIn`/`heldBy` above already use, per this
        // repo's `dev-journal/2026-08-12-design-context-answers.md` Q1.
        Quad::new(
            vocab::trigger(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_attribute()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::requirement_kind(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_attribute()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::effect_kind(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_attribute()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::effect_domain(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_attribute()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::requirement_locked_value(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_attribute()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::effect_locked_value(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_attribute()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::effect_step(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_attribute()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::requirement_threshold(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_attribute()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        // Self-declares which percept field each sensed predicate unlocks
        // -- replaces a hardcoded if-chain in `render::perceive_room` (see
        // `vocab::unlocks_field`'s own doc comment).
        Quad::new(
            vocab::dampness(),
            vocab::unlocks_field(),
            Term::Literal(lit_str("description")),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::decay(),
            vocab::unlocks_field(),
            Term::Literal(lit_str("description")),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::light(),
            vocab::unlocks_field(),
            Term::Literal(lit_str("description")),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::connects_to(),
            vocab::unlocks_field(),
            Term::Literal(lit_str("reachable")),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::connects_to(),
            vocab::unlocks_field(),
            Term::Literal(lit_str("exits")),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::contains(),
            vocab::unlocks_field(),
            Term::Literal(lit_str("items")),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            vocab::noticed_change(),
            vocab::unlocks_field(),
            Term::Literal(lit_str("noticedChange")),
            oxigraph::model::GraphName::DefaultGraph,
        ),
    ];
    let seed_produces = seed_quads
        .iter()
        .map(|q| format!("{q} ."))
        .collect::<Vec<_>>()
        .join("\n");
    let seed_commit = Commit {
        consumes: Vec::new(),
        produces: seed_produces,
        predicate: "genesis.seed".to_string(),
        via: None,
        responds_to: None,
        created_at: graph.now_ms().to_string(),
    };
    graph
        .apply_commit("demiurge", seed_commit)
        .expect("bootstrap content is always valid");

    // The Threshold is the one hand-authored, singular place in the game --
    // not a generated instance of a pattern -- but it carries no bespoke
    // prose of its own either: its description composes from these same
    // ground facts exactly the way any generated room's does (see
    // `render::compose_room_description`), so nothing has to special-case
    // it by identity elsewhere. Only its `name` is authored; dim (you
    // haven't earned the dark past it yet), otherwise unremarkable, and
    // already visited once by virtue of starting there. Minted via
    // `apply_commit` too now, `via` the seed above -- same reasoning.
    let start_raw = graph.fresh("room/");
    let start = vocab::foreign_uri_node(start_raw.as_str());
    let player_raw = graph.fresh("player/");
    let player = vocab::foreign_uri_node(player_raw.as_str());
    let content_quads = vec![
        Quad::new(
            start.clone(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_room()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            start.clone(),
            vocab::name(),
            Term::Literal(lit_str("The Threshold")),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            start.clone(),
            vocab::dampness(),
            Term::Literal(lit_float(0.0)),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            start.clone(),
            vocab::decay(),
            Term::Literal(lit_float(0.0)),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            start.clone(),
            vocab::light(),
            Term::Literal(lit_float(0.4)),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            start.clone(),
            vocab::visits(),
            Term::Literal(lit_int(1)),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            player.clone(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_player()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            start.clone(),
            vocab::contains(),
            Term::NamedNode(player.clone()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
    ];
    let content_produces = content_quads
        .iter()
        .map(|q| format!("{q} ."))
        .collect::<Vec<_>>()
        .join("\n");
    let content_commit = Commit {
        consumes: Vec::new(),
        produces: content_produces,
        predicate: "genesis.threshold".to_string(),
        via: Some(StrongRef {
            uri: seed_raw.as_str().to_string(),
            cid: "local".to_string(),
        }),
        responds_to: None,
        created_at: graph.now_ms().to_string(),
    };
    graph
        .apply_commit("demiurge", content_commit)
        .expect("bootstrap content is always valid");

    // Hands: universal body-verbs. Unconditional, no Requirement/Effect of
    // their own -- game.rs special-cases what firing them does (move the
    // player, alter inventory) rather than expressing "go" as an attribute
    // change, since there isn't a single attribute that models it.
    let hands = graph.fresh("machine/");
    let mut hands_delta = Delta::new()
        .assert(hands.clone(), vocab::rdf_type(), vocab::class_machine())
        .assert(player.clone(), vocab::equips(), hands.clone());
    for verb in ["go", "take", "drop", "inventory", "look", "examine"] {
        hands_delta = hands_delta.assert(hands.clone(), vocab::trigger(), lit_str(verb));
    }
    graph
        .commit("demiurge", hands_delta)
        .expect("hands machine is always valid");

    let (_, sight_delta) = build_sense_machine(graph, &player, &["room", "examine"], &sight_senses());
    graph
        .commit("demiurge", sight_delta)
        .expect("sight machine is always valid");

    // The map machine: a second, independent sense-machine rather than
    // another render_kind on the sight machine. Equipped here (alongside
    // sight, not conditionally) so the pipeline is real and testable
    // end-to-end; *how* a player earns or loses the ability to see a map,
    // as opposed to always having it from the start the way sight is fixed
    // here, is a separate design question this bootstrap doesn't answer.
    let (_, map_delta) = build_sense_machine(graph, &player, &["map"], &map_senses());
    graph
        .commit("demiurge", map_delta)
        .expect("map machine is always valid");

    Bootstrap {
        player,
        start_room: start,
        seed,
    }
}

/// The predicates the sight machine has always meant to sense, shared
/// between `bootstrap` and `ensure_sense_machines` below so the two can't
/// silently drift apart into two different definitions of "sight."
fn sight_senses() -> Vec<NamedNode> {
    vec![
        vocab::name(),
        vocab::contains(),
        vocab::connects_to(),
        vocab::direction(),
        vocab::locked(),
        vocab::wear(),
        vocab::dampness(),
        vocab::decay(),
        vocab::light(),
        vocab::visits(),
        vocab::noticed_change(),
    ]
}

/// The predicates the map machine has always meant to sense -- see
/// `sight_senses`'s own doc comment for why this is factored out the same
/// way.
fn map_senses() -> Vec<NamedNode> {
    vec![vocab::name(), vocab::visits()]
}

/// Equips `player` with sight/map sense-machines if it doesn't already have
/// them -- a no-op for any session bootstrapped under the current code,
/// since `bootstrap` already equips both. What it's actually for: a session
/// that was playing *before* `renderKind` existed at all. Its sight machine
/// still has `senses` but never got a `renderKind` triple, since the
/// predicate didn't exist yet when it was equipped -- `sense_machines_
/// for_kind` can never match a machine with no `renderKind` at all, so
/// without this, `perceive_room`/`perceive_map` return `None` forever for
/// that player and every room renders as "You perceive nothing," despite
/// the player, the room, and everything else about their world being
/// completely intact. Called on every snapshot load (`Game::from_snapshot`)
/// rather than folded into `bootstrap` itself, since bootstrap only ever
/// runs once, at genesis, and this has to run every time a session that
/// might predate this pipeline gets loaded.
pub fn ensure_sense_machines(graph: &mut WorldGraph, player: &NamedNode) {
    if machine::sense_machines_for_kind(graph, player, "room").is_empty() {
        let (_, sight_delta) = build_sense_machine(graph, player, &["room", "examine"], &sight_senses());
        graph
            .commit("demiurge", sight_delta)
            .expect("sight machine is always valid");
    }
    if machine::sense_machines_for_kind(graph, player, "map").is_empty() {
        let (_, map_delta) = build_sense_machine(graph, player, &["map"], &map_senses());
        graph
            .commit("demiurge", map_delta)
            .expect("map machine is always valid");
    }
}

fn seed_for(world_seed: u64, origin: &NamedNode, dir: Direction) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    world_seed.hash(&mut h);
    origin.as_str().hash(&mut h);
    (dir as u8).hash(&mut h);
    h.finish()
}

/// Fill a frontier point: a new room, its connecting edges, incidental
/// items, and -- some of the time -- a locked edge paired with a
/// mechanism whose drift and threshold machines are built from the same
/// `Requirement`/`Effect` vocabulary any future agency's proposals would
/// use. Returns the new room's id.
pub fn generate_frontier(
    graph: &mut WorldGraph,
    world_seed: u64,
    origin: &NamedNode,
    dir: Direction,
) -> NamedNode {
    // Which Theos governs this expansion -- origin's existing generator if
    // it has one, or a freshly rolled one equipped there on the spot (see
    // `ensure_theos_generator`'s own doc comment for why that's how
    // territory ends up reading as regions rather than per-room noise).
    let domain = ensure_theos_generator(graph, world_seed, origin);

    let mut rng = Rng::new(seed_for(world_seed, origin, dir));

    let new_room = graph.fresh("room/");
    let noun = rng.pick(room_nouns_for(&domain));

    // The ground facts -- rolled once, stored, and everything else about
    // how this room presents (its name's adjective, its description) is
    // derived from them rather than being its own independent roll. No
    // `description` literal gets asserted for this room at all: render_room
    // composes it fresh from these facts every call.
    let dampness = rng.gen_float();
    let decay = rng.gen_float();
    let light = rng.gen_float();
    let (dominant, intensity) = character::dominant_trait(dampness, decay, light);
    let adj = character::adjective(dominant, intensity);
    let name = format!("{} {}", capitalize(adj), capitalize(noun));

    let edge_fwd = graph.fresh("edge/");
    let edge_back = graph.fresh("edge/");

    let mut d = Delta::new()
        .assert(new_room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(new_room.clone(), vocab::name(), lit_str(name))
        .assert(new_room.clone(), vocab::dampness(), lit_float(dampness))
        .assert(new_room.clone(), vocab::decay(), lit_float(decay))
        .assert(new_room.clone(), vocab::light(), lit_float(light))
        .assert(new_room.clone(), vocab::visits(), lit_int(0))
        .assert(edge_fwd.clone(), vocab::rdf_type(), vocab::class_edge())
        .assert(edge_fwd.clone(), vocab::to(), new_room.clone())
        .assert(edge_fwd.clone(), vocab::direction(), lit_str(dir.word()))
        .assert(origin.clone(), vocab::connects_to(), edge_fwd.clone())
        .assert(edge_back.clone(), vocab::rdf_type(), vocab::class_edge())
        .assert(edge_back.clone(), vocab::to(), origin.clone())
        .assert(
            edge_back.clone(),
            vocab::direction(),
            lit_str(dir.opposite().word()),
        )
        .assert(new_room.clone(), vocab::connects_to(), edge_back.clone());

    let locked = rng.chance(30);
    if locked {
        d = d.assert(edge_fwd.clone(), vocab::locked(), lit_bool(true));

        let lever = graph.fresh("item/");
        d = d
            .assert(lever.clone(), vocab::rdf_type(), vocab::class_item())
            .assert(lever.clone(), vocab::name(), lit_str("mechanism"))
            .assert(lever.clone(), vocab::wear(), lit_float(0.0))
            .assert(origin.clone(), vocab::contains(), lever.clone());

        graph
            .commit("demiurge", d)
            .expect("frontier room + mechanism content is always valid");

        let (_, drift_delta) = build_action_machine(
            graph,
            &lever,
            "pull",
            &[Requirement::EdgeLocked {
                edge: edge_fwd.clone(),
                equals: true,
            }],
            &Effect::IncrementAttr {
                node: lever.clone(),
                attr: vocab::wear(),
                step: LEVER_PULL_STEP,
            },
        );
        graph
            .commit("demiurge", drift_delta)
            .expect("drift machine is always valid");

        let (_, threshold_delta) = build_action_machine(
            graph,
            &lever,
            "pull",
            &[
                Requirement::EdgeLocked {
                    edge: edge_fwd.clone(),
                    equals: true,
                },
                Requirement::AttrAtLeast {
                    node: lever.clone(),
                    attr: vocab::wear(),
                    threshold: LEVER_WEAR_THRESHOLD,
                },
            ],
            &Effect::SetEdgeLocked {
                edge: edge_fwd.clone(),
                value: false,
            },
        );
        graph
            .commit("demiurge", threshold_delta)
            .expect("threshold machine is always valid");

        // Congruence-detection stand-in: this call site is the one place
        // that produces the "pull, drift, then threshold-gated unlock"
        // shape, so the signature is hardcoded here rather than computed
        // by comparing arbitrary machine pairs for interface congruence.
        // The crystallization *mechanism* (recur once, earn a shared kind
        // and name) is real; the *matching* that would let it generalize
        // to shapes nobody wrote a call site for is the honest gap left
        // for later.
        crystallize(graph, &lever, "pull:incrementAttr+setEdgeLocked", "lever");
    } else {
        graph
            .commit("demiurge", d)
            .expect("frontier room content is always valid");
    }

    // Propagate dominion: the room just minted is this same Theos's
    // territory too, so the *next* expansion outward from it (in any
    // direction) reads the domain back here instead of rolling a fresh
    // one -- this is the actual mechanism that produces coherent regions.
    let (_, propagate_delta) = build_theos_generator(graph, &new_room, &domain);
    graph
        .commit("demiurge", propagate_delta)
        .expect("theos generator machine is always valid");

    let item_count = rng.gen_range(3);
    for _ in 0..item_count {
        let item = graph.fresh("item/");
        let noun = rng.pick(item_nouns_for(&domain));
        let d = Delta::new()
            .assert(item.clone(), vocab::rdf_type(), vocab::class_item())
            .assert(item.clone(), vocab::name(), lit_str(noun.to_string()))
            .assert(item.clone(), vocab::portable(), lit_bool(true))
            .assert(new_room.clone(), vocab::contains(), item.clone());
        graph
            .commit("demiurge", d)
            .expect("flavor item content is always valid");
    }

    new_room
}

fn class_for_signature(sig: &str) -> NamedNode {
    let clean: String = sig
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    oxigraph::model::NamedNode::new(format!("http://ww/class/sig-{clean}"))
        .expect("sanitized signature is a well-formed IRI segment")
}

/// The first occurrence of a signature stays a singular haecceity: real,
/// committed, but nameless. Only on recurrence does it earn a shared class
/// and a name -- and only from that point forward; the first instance is
/// never retroactively renamed.
fn crystallize(graph: &mut WorldGraph, item: &NamedNode, signature: &str, synthesized_name: &str) {
    let class = class_for_signature(signature);
    let current = graph
        .object(&class, &vocab::seen_count())
        .and_then(|t| as_string(&t))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let next = current + 1;

    let mut d = Delta::new();
    if current > 0 {
        d = d.retract(class.clone(), vocab::seen_count(), lit_int(current));
    }
    d = d.assert(class.clone(), vocab::seen_count(), lit_int(next));

    if next >= 2 {
        if graph.object(&class, &vocab::name()).is_none() {
            d = d.assert(class.clone(), vocab::name(), lit_str(synthesized_name));
        }
        d = d.assert(item.clone(), vocab::rdf_type(), class.clone());
        if let Some(old_name) = graph.object(item, &vocab::name()) {
            d = d.retract(item.clone(), vocab::name(), old_name);
        }
        d = d.assert(item.clone(), vocab::name(), lit_str(synthesized_name));
    }

    graph
        .commit("demiurge", d)
        .expect("crystallization bookkeeping is always valid");
}
