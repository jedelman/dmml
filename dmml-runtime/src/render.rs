//! Prose derived fresh from the graph on every call, same discipline as
//! before -- now built through `Percept` (see `dmml_runtime::percept`) rather
//! than straight to a `String`. A sense-machine's operation
//! (`perceive_room`/`perceive_map`/`perceive_examine`) produces the
//! structured percept, gated by what it declares it senses;
//! `render_percept_text` is this crate's one
//! renderer over that structure -- the reference `cli/` uses directly, and
//! what a web (or future TUI/WebGL/image) frontend's own renderer is
//! checked against. Also still home to two introspection views
//! (`render_assemblage`, `render_kinds`) that exist specifically so the
//! crystallization mechanism can be watched operating from inside the game,
//! not just trusted to work -- those stay plain strings, not percepts:
//! they're debug views of the graph itself, not something any in-world
//! sense-machine perceives.

use std::collections::HashSet;
use std::sync::OnceLock;

use oxigraph::model::{NamedNode, Term};
use oxigraph::sparql::{PreparedSparqlQuery, SparqlEvaluator};

use crate::character;
use crate::direction::Direction;
use crate::graph::{as_float, as_int, as_string, short, term_str, WorldGraph};
use crate::machine::{self, Effect};
use crate::percept::{Percept, PerceptValue};
use crate::vocab;

pub fn display_name(graph: &WorldGraph, node: &NamedNode) -> String {
    graph
        .object(node, &vocab::name())
        .and_then(|t| as_string(&t))
        .unwrap_or_else(|| "an unnamed thing".to_string())
}

/// A response to firing a machine's effect, composed from the effect's own
/// shape and its resulting value -- not authored per-machine. Same
/// discipline as `character.rs`'s trait -> clause step, generalized past a
/// room's own facts to whatever an `Effect` touches: an `IncrementAttr`
/// reads the attribute back post-commit and bands it by fixed absolute
/// thresholds, the same convention `perceive_examine`'s wear band and
/// `character.rs`'s own intensity bands already use -- deliberately *not*
/// proportional to the attribute's declared `graded_range` ceiling, which
/// is clamp headroom, not a "fullness" scale (`wear`'s ceiling is 2.0, but
/// every machine that actually gates on it, like the lever's threshold,
/// means 1.0); a `SetEdgeLocked` reads which way the lock just flipped. A
/// machine says what kind of thing just happened, never its own authored
/// sentence.
pub fn describe_effect_outcome(graph: &WorldGraph, effect: &Effect) -> String {
    match effect {
        Effect::IncrementAttr { node, attr, .. } => {
            let value = graph.object(node, attr).and_then(|t| as_float(&t)).unwrap_or(0.0);
            // Deliberately never says "gives way" -- that phrase is
            // reserved for an edge actually unlocking below, so the two
            // kinds of event never read as the same thing on a turn where
            // both happen to fire together.
            match value {
                v if v >= 1.0 => "It won't take any more.",
                v if v >= 0.67 => "It strains, close to giving.",
                v if v >= 0.34 => "It gives a little more than before.",
                v if v > 0.0 => "It has moved, but barely.",
                _ => "Nothing gives.",
            }
            .to_string()
        }
        Effect::SetEdgeLocked { value, .. } => if *value {
            "Somewhere, a door seals shut."
        } else {
            "Somewhere, a door gives way."
        }
        .to_string(),
        // Never reaches a player as a fired-effect message -- generation
        // is silent structure, described by entering the room it produced
        // (`render_room_text`), not by a "you did X" line the way pulling
        // a lever is. See `Effect::GenerateFrontier`'s own doc comment.
        Effect::GenerateFrontier { .. } => String::new(),
    }
}

/// Does any predicate in `senses` self-declare (via `unlocksField`) that it
/// exposes `field`? Replaces a hardcoded `senses.contains(&vocab::X())`
/// check with a lookup against genesis-declared content (see
/// `demiurge::bootstrap`) -- adding or changing which sense unlocks which
/// percept field is now a content change, not a Rust change.
fn senses_unlock(graph: &WorldGraph, senses: &HashSet<NamedNode>, field: &str) -> bool {
    senses.iter().any(|s| {
        graph
            .objects(s, &vocab::unlocks_field())
            .iter()
            .any(|t| as_string(t).as_deref() == Some(field))
    })
}

fn as_node_owned(t: Term) -> Option<NamedNode> {
    match t {
        Term::NamedNode(n) => Some(n),
        _ => None,
    }
}

/// A `Drift` node's existence, composed into a line at render time --
/// never stored. There's no narrated specificity to draw on anymore (that
/// was always an AI eyeballing two JSON blobs and inventing plausible
/// prose, exactly the "prose as information" pattern this pipeline exists
/// to avoid) -- only the structured fact that a foreign correspondence's
/// CID changed, which is real but doesn't say *what* changed. Honest
/// about that rather than pretending otherwise.
fn drift_commentary() -> String {
    "Something about this place has shifted since you last reached it -- \
     you can't say exactly what."
        .to_string()
}

/// The sight-machine's operation: `room`'s own facts, gated field-by-field
/// by the union of `senses` across whatever machine(s) `player` has
/// equipped with `renderKind "room"`. `None` means no such machine is
/// equipped at all -- nothing perceivable, not a fallback to perceiving
/// everything.
pub fn perceive_room(
    graph: &WorldGraph,
    player: &NamedNode,
    room: &NamedNode,
    unexplored: &[Direction],
) -> Option<Percept> {
    let machines = machine::sense_machines_for_kind(graph, player, "room");
    if machines.is_empty() {
        return None;
    }
    let senses = machine::sensed_predicates(graph, &machines);

    let name = graph
        .object(room, &vocab::name())
        .and_then(|t| as_string(&t))
        .unwrap_or_else(|| "an unnamed place".to_string());
    let mut p = Percept::new("room", name);

    // No `description` predicate exists to gate on anymore -- a room's
    // description is always composed fresh from its material facts
    // (dampness/decay/light), never stored, so the gate is "does this
    // sense-machine perceive any of the facts it's composed from" instead
    // of a single stand-in predicate. Which facts unlock `"description"`
    // is itself content now (`vocab::unlocks_field`, declared in
    // `demiurge::bootstrap`), not hardcoded here.
    if senses_unlock(graph, &senses, "description") {
        p = p.with_field(
            "description",
            PerceptValue::Text(compose_room_description(graph, room)),
        );
    }

    // What's reachable through an open door -- computed once, up front, if
    // connectsTo is sensed at all (no sense of edges at all means no way
    // to know what's beyond one). Expanded perception, not a separate
    // capability: the fields below fold an adjacent room's own facts in
    // alongside `room`'s, tagged by direction, using the exact same
    // senses-gated logic they already use for `room` itself -- nothing
    // peek-specific lives here. Gated on its own `"reachable"` key, not
    // `"exits"` -- both happen to be unlocked by the same `connectsTo`
    // sense today (see `demiurge::bootstrap`), but they're conceptually
    // distinct (an internal computation the items/npcs-folding and exits
    // blocks both consume, vs. the exits percept field itself), and a
    // shared key would force them to move in lockstep for no real reason
    // if a future sense ever wants to unlock one without the other.
    let reachable = if senses_unlock(graph, &senses, "reachable") {
        reachable_adjacent_rooms(graph, room)
    } else {
        Vec::new()
    };

    if senses_unlock(graph, &senses, "items") {
        // Sorted by creation_order before mapping to display strings,
        // matching the precedent `machine.rs`'s `machines_for_verb`/
        // `sense_machines_for_kind` already set. Without this, ordering
        // depended on the store's own internal quad iteration order --
        // not stable across a snapshot dump/reload round trip, confirmed
        // by reproduction (a restored session rendered the same two items
        // in a different order than the original).
        let mut item_nodes: Vec<NamedNode> = graph
            .objects(room, &vocab::contains())
            .into_iter()
            .filter_map(as_node_owned)
            .filter(|n| graph.has_type(n, &vocab::class_item()))
            .collect();
        item_nodes.sort_by_key(|n| crate::graph::creation_order(graph, n));
        let mut items: Vec<String> = item_nodes.into_iter().map(|n| display_name(graph, &n)).collect();

        // Npcs are never part of the deterministic local generator
        // (bootstrap only ever creates the Player; `demiurge::
        // generate_frontier` never mints one) -- the demiurge's Workers
        // AI-sourced commune responses are the one source of them so far.
        // A separate field from items since "you see" and "also here" read
        // differently for an inert thing versus someone.
        let mut npc_nodes: Vec<NamedNode> = graph
            .objects(room, &vocab::contains())
            .into_iter()
            .filter_map(as_node_owned)
            .filter(|n| graph.has_type(n, &vocab::class_npc()))
            .collect();
        npc_nodes.sort_by_key(|n| crate::graph::creation_order(graph, n));
        let mut npcs: Vec<String> = npc_nodes.into_iter().map(|n| display_name(graph, &n)).collect();

        for (dir, adjacent) in &reachable {
            for n in graph
                .objects(adjacent, &vocab::contains())
                .into_iter()
                .filter_map(as_node_owned)
            {
                if graph.has_type(&n, &vocab::class_item()) {
                    items.push(format!("{} (to the {dir})", display_name(graph, &n)));
                } else if graph.has_type(&n, &vocab::class_npc()) {
                    npcs.push(format!("{} (to the {dir})", display_name(graph, &n)));
                }
            }
        }

        if !items.is_empty() {
            p = p.with_field("items", PerceptValue::List(items));
        }
        if !npcs.is_empty() {
            p = p.with_field("npcs", PerceptValue::List(npcs));
        }
    }

    if senses_unlock(graph, &senses, "exits") {
        let exits: Vec<String> = graph
            .objects(room, &vocab::connects_to())
            .into_iter()
            .filter_map(as_node_owned)
            .filter_map(|edge| {
                graph
                    .object(&edge, &vocab::direction())
                    .and_then(|t| as_string(&t))
            })
            .collect();
        p = p.with_field("exits", PerceptValue::List(exits));

        let unexplored_words: Vec<String> =
            unexplored.iter().map(|d| d.word().to_string()).collect();
        if !unexplored_words.is_empty() {
            p = p.with_field("unexplored", PerceptValue::List(unexplored_words));
        }
    }

    if senses_unlock(graph, &senses, "noticedChange") {
        // Corruption as content: this room's foreign correspondence has
        // drifted (see dmml_runtime::vocab::noticed_change, Game::reach). Each
        // accreted fact is a `Drift` node -- old CID, new CID, when
        // observed -- never a narrated sentence, so "what changed" is
        // composed fresh from those facts at render time
        // (`drift_commentary`), not stored. Never touches `description`
        // above: the room's own ground truth is untouched by what a
        // foreign source claims changed.
        let has_drift = |n: &NamedNode| graph.has_type(n, &vocab::class_drift());
        let mut noticed: Vec<String> = Vec::new();
        if graph
            .objects(room, &vocab::noticed_change())
            .into_iter()
            .filter_map(as_node_owned)
            .any(|n| has_drift(&n))
        {
            noticed.push(drift_commentary());
        }

        for (dir, adjacent) in &reachable {
            if graph
                .objects(adjacent, &vocab::noticed_change())
                .into_iter()
                .filter_map(as_node_owned)
                .any(|n| has_drift(&n))
            {
                noticed.push(format!("{} (to the {dir})", drift_commentary()));
            }
        }

        if !noticed.is_empty() {
            p = p.with_field("noticedChange", PerceptValue::List(noticed));
        }
    }

    Some(p)
}

// `?room` is projected in the SELECT list even though `graph::query_bound`
// substitutes it before execution and no caller reads it back out of a
// solution -- `PreparedSparqlQuery::substitute_variable` errors with
// `NotExistingSubstitutedVariable` on a variable the query optimizer
// decided was safe to drop for not being an output, which a subject-only,
// single-use join variable like this one otherwise is. Projecting it costs
// nothing and keeps it live for substitution to find.
const REACHABLE_ADJACENT_SPARQL: &str = "\
PREFIX ww: <http://ww/>
SELECT ?room ?dir ?adjacent WHERE {
  ?room ww:connectsTo ?edge .
  ?edge ww:direction ?dir ; ww:to ?adjacent .
  FILTER NOT EXISTS { ?edge ww:locked true }
}";

/// Parsed exactly once, reused on every call -- see `graph::query_bound`
/// and this module's own doc comment on why this is the one place this
/// crate reaches for real SPARQL: which rooms are reachable through an
/// unlocked edge is a genuine join (room -> edge -> adjacent room), not a
/// single subject's own triples the existing pattern-lookup API already
/// handles fine. Deliberately doesn't touch *what's true* about an
/// adjacent room -- that's still the job of the same field-building code
/// `perceive_room` already runs on `room` itself, just handed a second
/// subject to run on too. See `reachable_adjacent_rooms`.
fn reachable_query() -> &'static PreparedSparqlQuery {
    static QUERY: OnceLock<PreparedSparqlQuery> = OnceLock::new();
    QUERY.get_or_init(|| {
        SparqlEvaluator::new()
            .parse_query(REACHABLE_ADJACENT_SPARQL)
            .expect("REACHABLE_ADJACENT_SPARQL is valid by construction")
    })
}

/// Every room reachable from `room` by a single unlocked edge, with the
/// direction that reaches it -- "expanded perception" is this: knowing
/// what's reachable, not a second, bespoke interpretation of what's found
/// there. What a peek actually reveals is entirely up to whichever of
/// `perceive_room`'s own senses-gated fields choose to fold an adjacent
/// room's facts in alongside `room`'s own, tagged by direction -- there's
/// no separate peek vocabulary here, no hardcoded predicate-to-sentence
/// table. A predicate this function's caller has no field for at all (a
/// room's bare `light` level, say) simply isn't peekable, the same honest
/// way it wouldn't be perceivable at all if `senses` didn't cover it.
fn reachable_adjacent_rooms(graph: &WorldGraph, room: &NamedNode) -> Vec<(String, NamedNode)> {
    let mut out = Vec::new();
    for solution in graph.query_bound(reachable_query().clone(), room.clone()) {
        let Some(Term::Literal(dir)) = solution.get("dir") else {
            continue;
        };
        let Some(Term::NamedNode(adjacent)) = solution.get("adjacent") else {
            continue;
        };
        out.push((dir.value().to_string(), adjacent.clone()));
    }
    out
}

/// The sight-machine's other operation: a close look at `target` (an item,
/// typically), gated the same way `perceive_room` gates a room -- collapsed
/// into the same sense-machine as room perception (see `demiurge::
/// bootstrap`'s sight machine declaring both `"room"` and `"examine"` as
/// `render_kind`s) rather than a bespoke, ungated function the way this
/// used to work. `None` if `player` has no `"examine"`-capable machine
/// equipped at all. An item carries no descriptive prose of its own --
/// only `wear`, for the one item kind (a lever) that has a graded fact to
/// compose from -- so a plain item with nothing sensed beyond its own
/// existence yields `Some` with no fields at all; the text renderer's
/// fallback (`render_examine_percept_text`) is what a player actually sees
/// for it, composed from the item's name, not stored as its own literal.
pub fn perceive_examine(graph: &WorldGraph, player: &NamedNode, target: &NamedNode) -> Option<Percept> {
    let machines = machine::sense_machines_for_kind(graph, player, "examine");
    if machines.is_empty() {
        return None;
    }
    let senses = machine::sensed_predicates(graph, &machines);

    let mut p = Percept::new("examine", display_name(graph, target));

    if senses.contains(&vocab::wear()) {
        if let Some(wear) = graph.object(target, &vocab::wear()).and_then(|t| as_float(&t)) {
            let band = match wear {
                w if w >= 1.0 => "It hangs loose now, whatever resisted it spent.",
                w if w >= 0.67 => "It strains against something inside the wall, close to giving.",
                w if w >= 0.34 => "It gives a little more each time it's pulled.",
                w if w > 0.0 => "It has been pulled, but barely moved.",
                _ => "It hasn't been touched.",
            };
            p = p.with_field("wear", PerceptValue::Text(band.to_string()));
        }
    }

    Some(p)
}

/// The map machine's operation: every `Room` the player has visited
/// (`visits >= 1`), each perceived as its own small `roomSummary` percept --
/// nested inside the map percept, not a bespoke summary type outside this
/// vocabulary. Gated by `player`'s `renderKind "map"` machine(s) the same
/// way `perceive_room` is gated by its own `"room"` machine(s); `None` if
/// none is equipped.
pub fn perceive_map(graph: &WorldGraph, player: &NamedNode) -> Option<Percept> {
    let machines = machine::sense_machines_for_kind(graph, player, "map");
    if machines.is_empty() {
        return None;
    }
    let senses = machine::sensed_predicates(graph, &machines);

    let current_room = graph
        .subjects(&vocab::contains(), &Term::NamedNode(player.clone()))
        .into_iter()
        .next();

    let mut rooms: Vec<(NamedNode, Percept)> = graph
        .subjects(&vocab::rdf_type(), &Term::NamedNode(vocab::class_room()))
        .into_iter()
        .filter_map(|room| {
            let visits = if senses.contains(&vocab::visits()) {
                graph
                    .object(&room, &vocab::visits())
                    .and_then(|t| as_int(&t))
            } else {
                None
            }
            .unwrap_or(0);
            if visits < 1 {
                return None;
            }

            let name = if senses.contains(&vocab::name()) {
                graph
                    .object(&room, &vocab::name())
                    .and_then(|t| as_string(&t))
            } else {
                None
            }
            .unwrap_or_else(|| "an unnamed place".to_string());

            let mut summary = Percept::new("roomSummary", name)
                .with_field("visits", PerceptValue::Text(visits.to_string()));
            if current_room.as_ref() == Some(&room) {
                summary = summary.with_field("current", PerceptValue::Text("here".to_string()));
            }
            Some((room, summary))
        })
        .collect();
    rooms.sort_by_key(|(room, _)| crate::graph::creation_order(graph, room));

    Some(
        Percept::new("map", "Map").with_field(
            "rooms",
            PerceptValue::Nested(rooms.into_iter().map(|(_, p)| p).collect()),
        ),
    )
}

/// This crate's one renderer over a `Percept`: what `cli/` uses directly,
/// and the reference every other frontend's own renderer (web's HTML,
/// eventually a TUI's or a WebGL scene's) produces the same information as,
/// from the identical structure -- see `dmml_runtime::percept`'s module doc.
pub fn render_percept_text(p: &Percept) -> String {
    match p.kind.as_str() {
        "room" => render_room_percept_text(p),
        "map" => render_map_percept_text(p),
        "examine" => render_examine_percept_text(p),
        _ => render_generic_percept_text(p),
    }
}

fn render_room_percept_text(p: &Percept) -> String {
    let mut out = format!("== {} ==", p.title);
    if let Some(PerceptValue::Text(description)) = p.field("description") {
        out.push('\n');
        out.push_str(description);
    }
    if let Some(PerceptValue::List(items)) = p.field("items") {
        out.push_str("\nYou see: ");
        out.push_str(&items.join(", "));
    }
    if let Some(PerceptValue::List(npcs)) = p.field("npcs") {
        out.push_str("\nAlso here: ");
        out.push_str(&npcs.join(", "));
    }
    match p.field("exits") {
        Some(PerceptValue::List(exits)) if !exits.is_empty() => {
            out.push_str("\nExits: ");
            out.push_str(&exits.join(", "));
        }
        Some(PerceptValue::List(_)) => out.push_str("\nThere are no confirmed exits."),
        _ => {}
    }
    if let Some(PerceptValue::List(unexplored)) = p.field("unexplored") {
        out.push_str("\nUnexplored: ");
        out.push_str(&unexplored.join(", "));
    }
    if let Some(PerceptValue::List(noticed)) = p.field("noticedChange") {
        for line in noticed {
            out.push('\n');
            out.push_str(line);
        }
    }
    out
}

fn render_map_percept_text(p: &Percept) -> String {
    let Some(PerceptValue::Nested(rooms)) = p.field("rooms") else {
        return format!("{}: nothing mapped yet.", p.title);
    };
    if rooms.is_empty() {
        return format!("{}: nothing mapped yet.", p.title);
    }
    let mut lines = vec![format!("{}:", p.title)];
    for room in rooms {
        let visits = match room.field("visits") {
            Some(PerceptValue::Text(v)) => v.as_str(),
            _ => "?",
        };
        let here = matches!(room.field("current"), Some(PerceptValue::Text(_)));
        lines.push(format!(
            "  {} -- visited {visits}x{}",
            room.title,
            if here { " (here)" } else { "" }
        ));
    }
    lines.join("\n")
}

fn render_examine_percept_text(p: &Percept) -> String {
    // No stored description to fall back on -- an item with no graded
    // fact to compose from (the common case: most items only ever carry
    // a name) genuinely has nothing more to say about itself. Distinct
    // from "you have no way to examine anything" (perceive_examine
    // returning `None` entirely, handled one layer up in `Game::examine`)
    // -- this is "you looked, and there wasn't more to make out."
    match p.field("wear") {
        Some(PerceptValue::Text(band)) => band.clone(),
        _ => format!("Nothing more to notice about the {}.", p.title),
    }
}

/// A total fallback so `render_percept_text` never panics on a percept kind
/// it doesn't have bespoke formatting for yet -- not currently reachable
/// (only "room" and "map" percepts exist), kept honest rather than a
/// `match` that assumes its own exhaustiveness.
fn render_generic_percept_text(p: &Percept) -> String {
    let mut lines = vec![p.title.clone()];
    for f in &p.fields {
        let rendered = match &f.value {
            PerceptValue::Text(t) => t.clone(),
            PerceptValue::List(items) => items.join(", "),
            PerceptValue::Nested(_) => "[...]".to_string(),
        };
        lines.push(format!("{}: {rendered}", f.key));
    }
    lines.join("\n")
}

/// `perceive_room` + `render_percept_text` in one call, with the "nothing
/// equipped" fallback text a caller needs -- what every room-facing call
/// site in `game.rs` actually wants, since none of them have a use for the
/// structured `Percept` itself (yet -- a server-side caller wanting the
/// wire-format JSON would call `perceive_room` directly instead).
pub fn render_room_text(
    graph: &WorldGraph,
    player: &NamedNode,
    room: &NamedNode,
    unexplored: &[Direction],
) -> String {
    match perceive_room(graph, player, room, unexplored) {
        Some(p) => render_percept_text(&p),
        None => "You perceive nothing.".to_string(),
    }
}

/// Reads `room`'s dampness/decay/light/visits and composes a description
/// from them -- nothing here is stored; call it again after any of those
/// facts change (crossing the wear-style dampness/decay bands, a return
/// visit) and the text changes with it. Two clauses: the room's material
/// character (via `character.rs`, the same vocabulary its name's adjective
/// was drawn from at generation time, so the two can't contradict each
/// other), and a clause for the player's own history with this specific
/// room -- durée, not just a snapshot of current facts.
fn compose_room_description(graph: &WorldGraph, room: &NamedNode) -> String {
    let dampness = graph
        .object(room, &vocab::dampness())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.0);
    let decay = graph
        .object(room, &vocab::decay())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.0);
    let light = graph
        .object(room, &vocab::light())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.5);
    let visits = graph
        .object(room, &vocab::visits())
        .and_then(|t| as_int(&t))
        .unwrap_or(0);

    let (dominant, intensity) = character::dominant_trait(dampness, decay, light);
    let material = character::clause(dominant, intensity);

    let history = match visits {
        0 | 1 => "You've never stood here before.",
        2 | 3 => "You've passed through here before.",
        _ => "You know this place well by now.",
    };

    format!("{material} {history}")
}

pub fn render_inventory(graph: &WorldGraph, player: &NamedNode) -> String {
    // `holds` is retired for `take`/`drop` -- see `vocab::held_by`'s doc
    // comment. What the player currently carries is now the materialized
    // `heldBy` state, read via `current_subjects_with` rather than a raw
    // pattern lookup (see `WorldGraph::current_value`'s own doc comment
    // for why a plain `objects`/`object` query can't answer this
    // correctly once a predicate's been re-asserted more than once).
    let items: Vec<String> = graph
        .current_subjects_with(&vocab::held_by(), &Term::NamedNode(player.clone()))
        .into_iter()
        .map(|n| display_name(graph, &n))
        .collect();
    if items.is_empty() {
        "You are carrying nothing.".to_string()
    } else {
        format!("You are carrying: {}", items.join(", "))
    }
}


/// Dump every triple touching `node`, in either direction. This is the
/// literal answer to "interrogate the world from within" -- a player-
/// visible view of the graph itself, not a curated rendering of it.
pub fn render_assemblage(graph: &WorldGraph, node: &NamedNode) -> String {
    let mut lines = vec![format!("Assemblage of {}:", short(node))];
    for (p, o) in graph.triples_with_subject(node) {
        lines.push(format!("  -> {} {}", short(&p), term_str(&o)));
    }
    for (p, s) in graph.triples_with_object(node) {
        lines.push(format!("  <- {} {}", short(&s), short(&p)));
    }
    lines.join("\n")
}

/// Every predicate the world has self-declared via `<p> rdf:type
/// ww:Relation|ww:Attribute` so far, with how many triples actually use it
/// -- the introspection view for genuinely new relation types, the same
/// "watch it operating from inside the game" ethos `render_kinds` already
/// gives crystallization.
pub fn render_relations(graph: &WorldGraph) -> String {
    let relations = graph.subjects(
        &vocab::rdf_type(),
        &Term::NamedNode(vocab::class_relation()),
    );
    let attributes = graph.subjects(
        &vocab::rdf_type(),
        &Term::NamedNode(vocab::class_attribute()),
    );
    if relations.is_empty() && attributes.is_empty() {
        return "No new relations have been declared -- everything so far uses the fixed vocabulary."
            .to_string();
    }
    let mut lines = vec!["Declared relation vocabulary:".to_string()];
    for p in relations {
        let uses = graph.all_with_predicate(&p).len();
        lines.push(format!("  {} (relation) -- used {uses} time(s)", short(&p)));
    }
    for p in attributes {
        let uses = graph.all_with_predicate(&p).len();
        lines.push(format!(
            "  {} (attribute) -- used {uses} time(s)",
            short(&p)
        ));
    }
    lines.join("\n")
}

pub fn render_kinds(graph: &WorldGraph) -> String {
    let counts = graph.all_with_predicate(&vocab::seen_count());
    if counts.is_empty() {
        return "No patterns have recurred yet -- nothing has crystallized.".to_string();
    }
    let mut lines = vec!["Crystallized / crystallizing kinds:".to_string()];
    for (class, count_term) in counts {
        let count = as_string(&count_term).unwrap_or_default();
        match graph
            .object(&class, &vocab::name())
            .and_then(|t| as_string(&t))
        {
            Some(n) => lines.push(format!("  \"{n}\" -- seen {count} time(s), crystallized")),
            None => lines.push(format!(
                "  {} -- seen {count} time(s), not yet crystallized",
                short(&class)
            )),
        }
    }
    lines.join("\n")
}
