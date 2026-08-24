//! Desiring-machines: the unified replacement for `TransitionRule` +
//! `Precondition` + `Effect`. A machine is a node (`rdf:type ww:Machine`)
//! equipped to a substance, carrying a trigger verb, zero or more
//! requirements, and an effect. Nothing here is a different *kind* of fact
//! from anything in `graph.rs` — a requirement or effect is just another
//! node with its own triples, read back the same way a room or item is.

use std::collections::HashSet;

use oxigraph::model::{NamedNode, Term};

use crate::graph::{
    as_bool, as_float, as_node, as_string, lit_bool, lit_float, lit_str, Delta, WorldGraph,
};
use crate::vocab;

#[derive(Debug, Clone)]
pub enum Requirement {
    PlayerInRoom {
        room: NamedNode,
    },
    EdgeLocked {
        edge: NamedNode,
        equals: bool,
    },
    AttrAtLeast {
        node: NamedNode,
        attr: NamedNode,
        threshold: f32,
    },
}

#[derive(Debug, Clone)]
pub enum Effect {
    IncrementAttr {
        node: NamedNode,
        attr: NamedNode,
        step: f32,
    },
    SetEdgeLocked {
        edge: NamedNode,
        value: bool,
    },
    /// A Theos's generative dominion, not a rule fired against a fixed
    /// target the way the two effects above are -- there's no single node
    /// this bears on, since what it governs (which room, which direction)
    /// is only known at the moment a player actually crosses an unmapped
    /// edge. Stored and read back through the identical Effect
    /// infrastructure as any other effect (so it's equally inspectable via
    /// `kinds`/`relations`, and equally something a future petition could
    /// propose), but *fired* directly by `demiurge::generate_frontier`
    /// reading it off whichever generator machine is equipped to the
    /// origin room -- not through `Game::fire_object_verb`'s verb-on-object
    /// dispatch, which stays exclusively for `IncrementAttr`/
    /// `SetEdgeLocked`. See `demiurge.rs`'s pantheon doc comment.
    GenerateFrontier {
        domain: String,
    },
}

/// Builds a verb-triggered machine (a drift rule, a threshold rule, or any
/// future single-purpose coupling) equipped to `owner`, and returns both
/// its id and the `Delta` that will bring it into existence once committed.
/// Carries no authored response text -- what firing it says is composed at
/// fire time from the `Effect`'s own shape and resulting value
/// (`render::describe_effect_outcome`), not stored per-machine prose.
pub fn build_action_machine(
    graph: &mut WorldGraph,
    owner: &NamedNode,
    verb: &str,
    requires: &[Requirement],
    effect: &Effect,
) -> (NamedNode, Delta) {
    let machine = graph.fresh("machine/");
    let mut d = Delta::new()
        .assert(machine.clone(), vocab::rdf_type(), vocab::class_machine())
        .assert(owner.clone(), vocab::equips(), machine.clone())
        .assert(machine.clone(), vocab::trigger(), lit_str(verb));

    for req in requires {
        let (req_node, req_delta) = build_requirement(graph, req);
        d.add.extend(req_delta.add);
        d = d.assert(machine.clone(), vocab::has_requirement(), req_node);
    }

    let (effect_node, effect_delta) = build_effect(graph, effect);
    d.add.extend(effect_delta.add);
    d = d.assert(machine.clone(), vocab::has_effect(), effect_node);

    (machine, d)
}

/// Builds a sensing machine: no trigger, no effect, a declared set of
/// predicates it makes traversable from whatever it's equipped to, and one
/// or more `render_kind`s naming which percept(s) its operation can produce
/// ("room", "map", "examine", ...) -- multi-valued the same way a
/// action-machine's `trigger` already is (see `build_action_machine`'s
/// `hands` caller in `demiurge.rs`), since one faculty (sight) perceiving
/// several different *kinds* of thing is the ordinary case, not a
/// coincidence needing a separate machine per kind. This is what answers
/// "what do I perceive, and as what" the same way `build_action_machine`
/// answers "what can I do."
pub fn build_sense_machine(
    graph: &mut WorldGraph,
    owner: &NamedNode,
    render_kinds: &[&str],
    senses: &[NamedNode],
) -> (NamedNode, Delta) {
    let machine = graph.fresh("machine/");
    let mut d = Delta::new()
        .assert(machine.clone(), vocab::rdf_type(), vocab::class_machine())
        .assert(owner.clone(), vocab::equips(), machine.clone());
    for kind in render_kinds {
        d = d.assert(machine.clone(), vocab::render_kind(), lit_str(*kind));
    }
    for p in senses {
        d = d.assert(machine.clone(), vocab::senses(), p.clone());
    }
    (machine, d)
}

/// Machines equipped to `owner` that declare `kind` among their
/// `render_kind`s -- what `render::perceive_room`/`perceive_map`/
/// `perceive_examine` look up before building anything, the same way
/// `machines_for_verb` looks up action-machines by trigger. No matching
/// machine means no percept at all: perception is bounded by what's
/// equipped, not by whether a call site happens to exist.
pub fn sense_machines_for_kind(graph: &WorldGraph, owner: &NamedNode, kind: &str) -> Vec<NamedNode> {
    let mut machines: Vec<NamedNode> = graph
        .objects(owner, &vocab::equips())
        .into_iter()
        .filter_map(as_node)
        .filter(|m| {
            graph
                .objects(m, &vocab::render_kind())
                .iter()
                .any(|t| as_string(t).as_deref() == Some(kind))
        })
        .collect();
    machines.sort_by_key(|m| crate::graph::creation_order(graph, m));
    machines
}

/// The union of `senses` declared across `machines` -- what a percept's
/// construction is allowed to read. Empty if `machines` is empty, which is
/// exactly what "no sense-machine equipped for this kind" should mean:
/// nothing perceivable, not a fallback to perceiving everything.
pub fn sensed_predicates(graph: &WorldGraph, machines: &[NamedNode]) -> HashSet<NamedNode> {
    machines
        .iter()
        .flat_map(|m| graph.objects(m, &vocab::senses()))
        .filter_map(as_node)
        .collect()
}

fn build_requirement(graph: &mut WorldGraph, req: &Requirement) -> (NamedNode, Delta) {
    let node = graph.fresh("req/");
    let d = match req {
        Requirement::PlayerInRoom { room } => Delta::new()
            .assert(
                node.clone(),
                vocab::requirement_kind(),
                lit_str("playerInRoom"),
            )
            .assert(node.clone(), vocab::requirement_room(), room.clone()),
        Requirement::EdgeLocked { edge, equals } => Delta::new()
            .assert(
                node.clone(),
                vocab::requirement_kind(),
                lit_str("edgeLocked"),
            )
            .assert(node.clone(), vocab::requirement_edge(), edge.clone())
            .assert(
                node.clone(),
                vocab::requirement_locked_value(),
                lit_bool(*equals),
            ),
        Requirement::AttrAtLeast {
            node: target,
            attr,
            threshold,
        } => Delta::new()
            .assert(
                node.clone(),
                vocab::requirement_kind(),
                lit_str("attrAtLeast"),
            )
            .assert(node.clone(), vocab::requirement_attr_node(), target.clone())
            .assert(
                node.clone(),
                vocab::requirement_attr_predicate(),
                attr.clone(),
            )
            .assert(
                node.clone(),
                vocab::requirement_threshold(),
                lit_float(*threshold),
            ),
    };
    (node, d)
}

fn build_effect(graph: &mut WorldGraph, effect: &Effect) -> (NamedNode, Delta) {
    let node = graph.fresh("effect/");
    let d = match effect {
        Effect::IncrementAttr {
            node: target,
            attr,
            step,
        } => Delta::new()
            .assert(node.clone(), vocab::effect_kind(), lit_str("incrementAttr"))
            .assert(node.clone(), vocab::effect_target_node(), target.clone())
            .assert(node.clone(), vocab::effect_attr_predicate(), attr.clone())
            .assert(node.clone(), vocab::effect_step(), lit_float(*step)),
        Effect::SetEdgeLocked { edge, value } => Delta::new()
            .assert(node.clone(), vocab::effect_kind(), lit_str("setEdgeLocked"))
            .assert(node.clone(), vocab::effect_edge(), edge.clone())
            .assert(node.clone(), vocab::effect_locked_value(), lit_bool(*value)),
        Effect::GenerateFrontier { domain } => Delta::new()
            .assert(
                node.clone(),
                vocab::effect_kind(),
                lit_str("generateFrontier"),
            )
            .assert(node.clone(), vocab::effect_domain(), lit_str(domain)),
    };
    (node, d)
}

pub fn read_requirement(graph: &WorldGraph, node: &NamedNode) -> Option<Requirement> {
    let kind = as_string(&graph.object(node, &vocab::requirement_kind())?)?;
    match kind.as_str() {
        "playerInRoom" => Some(Requirement::PlayerInRoom {
            room: as_node(graph.object(node, &vocab::requirement_room())?)?,
        }),
        "edgeLocked" => Some(Requirement::EdgeLocked {
            edge: as_node(graph.object(node, &vocab::requirement_edge())?)?,
            equals: as_bool(&graph.object(node, &vocab::requirement_locked_value())?)?,
        }),
        "attrAtLeast" => Some(Requirement::AttrAtLeast {
            node: as_node(graph.object(node, &vocab::requirement_attr_node())?)?,
            attr: as_node(graph.object(node, &vocab::requirement_attr_predicate())?)?,
            threshold: as_float(&graph.object(node, &vocab::requirement_threshold())?)?,
        }),
        _ => None,
    }
}

pub fn read_effect(graph: &WorldGraph, node: &NamedNode) -> Option<Effect> {
    let kind = as_string(&graph.object(node, &vocab::effect_kind())?)?;
    match kind.as_str() {
        "incrementAttr" => Some(Effect::IncrementAttr {
            node: as_node(graph.object(node, &vocab::effect_target_node())?)?,
            attr: as_node(graph.object(node, &vocab::effect_attr_predicate())?)?,
            step: as_float(&graph.object(node, &vocab::effect_step())?)?,
        }),
        "setEdgeLocked" => Some(Effect::SetEdgeLocked {
            edge: as_node(graph.object(node, &vocab::effect_edge())?)?,
            value: as_bool(&graph.object(node, &vocab::effect_locked_value())?)?,
        }),
        "generateFrontier" => Some(Effect::GenerateFrontier {
            domain: as_string(&graph.object(node, &vocab::effect_domain())?)?,
        }),
        _ => None,
    }
}

/// Machines equipped to `owner` whose trigger matches `verb`, in creation
/// order.
pub fn machines_for_verb(graph: &WorldGraph, owner: &NamedNode, verb: &str) -> Vec<NamedNode> {
    let mut machines: Vec<NamedNode> = graph
        .objects(owner, &vocab::equips())
        .into_iter()
        .filter_map(as_node)
        .filter(|m| {
            graph
                .objects(m, &vocab::trigger())
                .iter()
                .any(|t| as_string(t).as_deref() == Some(verb))
        })
        .collect();
    machines.sort_by_key(|m| crate::graph::creation_order(graph, m));
    machines
}

pub fn requirement_met(graph: &WorldGraph, req: &Requirement, player_room: &NamedNode) -> bool {
    match req {
        Requirement::PlayerInRoom { room } => room == player_room,
        Requirement::EdgeLocked { edge, equals } => match graph.object(edge, &vocab::locked()) {
            Some(t) => as_bool(&t) == Some(*equals),
            None => false,
        },
        Requirement::AttrAtLeast {
            node,
            attr,
            threshold,
        } => match graph.object(node, attr) {
            Some(t) => as_float(&t).is_some_and(|v| v >= *threshold),
            None => false,
        },
    }
}

/// Term -> NamedNode convenience, used by callers matching on Effect targets.
pub fn term_node(t: &Term) -> Option<NamedNode> {
    match t {
        Term::NamedNode(n) => Some(n.clone()),
        _ => None,
    }
}
