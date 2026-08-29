//! A real Datalog replacement candidate for `machine::requirement_met` /
//! `machine::machines_for_verb`'s "are this machine's requirements
//! satisfied" check, built on `crepe` (semi-naive evaluation, stratified
//! negation, compiled to native Rust -- not an interpreter over an AST).
//!
//! Scope, stated honestly: this covers requirement evaluation only, not
//! `apply_commit`'s referential-integrity checks (those are genuinely
//! oxigraph pattern queries, not admissibility logic, and don't map onto
//! Datalog any better than they already work). Not wired into `game.rs`'s
//! live dispatch yet -- `machines_ready` below is validated for equivalence
//! against the existing hand-rolled `requirement_met` in this module's own
//! test, and that has to hold before any real cutover is worth doing.
//!
//! crepe requires every fact field to implement `Copy` (its own docs: "if
//! fields do not implement Copy, consider passing references instead") --
//! `NamedNode`/`String` don't, so node identity is interned to a `u32`
//! symbol via `SymbolTable` below, the standard Datalog technique, not a
//! workaround specific to this module. Float attribute values are lowered
//! to fixed-point `i64` (six decimal digits) for the same Copy-and-Ord
//! requirement; the graph's own oxigraph float terms stay the source of
//! truth, nothing here writes a value back.

use std::collections::HashMap;
use std::collections::HashSet;

use oxigraph::model::NamedNode;

use crate::graph::{as_bool, as_float, WorldGraph};
use crate::machine::{read_requirement, Requirement};
use crate::vocab;

const FIXED_POINT_SCALE: f64 = 1_000_000.0;

fn quantize(v: f32) -> i64 {
    (v as f64 * FIXED_POINT_SCALE).round() as i64
}

/// Interns node identities (their IRI string) to small `u32` symbols, since
/// crepe's fact fields must be `Copy` and a `NamedNode`/`String` isn't.
#[derive(Default)]
struct SymbolTable {
    by_str: HashMap<String, u32>,
    by_sym: Vec<NamedNode>,
}

impl SymbolTable {
    fn intern(&mut self, node: &NamedNode) -> u32 {
        if let Some(&s) = self.by_str.get(node.as_str()) {
            return s;
        }
        let s = self.by_sym.len() as u32;
        self.by_sym.push(node.clone());
        self.by_str.insert(node.as_str().to_string(), s);
        s
    }

    fn resolve(&self, s: u32) -> NamedNode {
        self.by_sym[s as usize].clone()
    }
}

use crepe::crepe;

crepe! {
    // Ground facts, extracted from the graph by `machines_ready` below.
    // All symbols are `u32`s from `SymbolTable` -- see module docs for why.
    // crepe's own parser doesn't accept visibility modifiers on these
    // struct declarations, so this whole block stays flat in this module
    // (not a submodule) -- `machines_ready` below reaches these directly
    // the same way crepe's own doc example does.
    @input
    struct HasRequirement(u32, u32); // (machine, requirement_node)

    @input
    struct ReqPlayerInRoom(u32, u32); // (requirement_node, room)
    @input
    struct ReqEdgeLocked(u32, u32, bool); // (requirement_node, edge, equals)
    @input
    struct ReqAttrAtLeast(u32, u32, u32, i64); // (requirement_node, node, attr, threshold_fp)

    @input
    struct PlayerRoom(u32); // singleton: the current room
    @input
    struct EdgeLockedState(u32, bool); // (edge, current locked value)
    @input
    struct AttrValue(u32, u32, i64); // (node, attr, current value_fp)

    // A machine node, derived just so `AllRequirementsMet` can range over
    // every machine that has at least one requirement -- a
    // requirement-free machine is out of scope for this check (matches
    // `requirement_met`'s own contract: it's never called with an empty
    // slice by `.all()`'s real call sites without an equipped machine to
    // begin with).
    struct Machine(u32);
    Machine(m) <- HasRequirement(m, _);

    @output
    struct RequirementMet(u32);

    RequirementMet(r) <- ReqPlayerInRoom(r, room), PlayerRoom(room);
    RequirementMet(r) <- ReqEdgeLocked(r, edge, equals), EdgeLockedState(edge, current), (current == equals);
    RequirementMet(r) <- ReqAttrAtLeast(r, node, attr, threshold), AttrValue(node, attr, value), (value >= threshold);

    @output
    struct UnmetRequirement(u32); // machine with >=1 unmet requirement

    UnmetRequirement(m) <- HasRequirement(m, r), !RequirementMet(r);

    @output
    struct AllRequirementsMet(u32);

    AllRequirementsMet(m) <- Machine(m), !UnmetRequirement(m);
}

/// Real Datalog-derived equivalent of calling `machine::requirement_met`
/// across every requirement of every equipped machine and keeping the ones
/// where every requirement holds. Built entirely from facts already
/// reachable via `graph.objects`/`read_requirement` -- no new graph state.
pub fn machines_ready(graph: &WorldGraph, player_room: &NamedNode) -> HashSet<NamedNode> {
    let mut sym = SymbolTable::default();
    let mut runtime = Crepe::new();
    runtime.extend([PlayerRoom(sym.intern(player_room))]);

    let mut seen_edges: HashSet<u32> = HashSet::new();
    let mut seen_attrs: HashSet<(u32, u32)> = HashSet::new();

    // Every (machine, requirement) edge in the graph, via the same
    // `has_requirement` predicate `machine.rs` itself reads -- not a fresh
    // traversal invented for this module.
    for (machine, req_term) in graph.all_with_predicate(&vocab::has_requirement()) {
        let machine_sym = sym.intern(&machine);
        let req_node = match req_term {
            oxigraph::model::Term::NamedNode(n) => n,
            _ => continue,
        };
        let req_sym = sym.intern(&req_node);
        runtime.extend([HasRequirement(machine_sym, req_sym)]);

        let Some(req) = read_requirement(graph, &req_node) else {
            continue;
        };
        match req {
            Requirement::PlayerInRoom { room } => {
                runtime.extend([ReqPlayerInRoom(req_sym, sym.intern(&room))]);
            }
            Requirement::EdgeLocked { edge, equals } => {
                let edge_sym = sym.intern(&edge);
                runtime.extend([ReqEdgeLocked(req_sym, edge_sym, equals)]);
                if seen_edges.insert(edge_sym) {
                    if let Some(t) = graph.object(&edge, &vocab::locked()) {
                        if let Some(b) = as_bool(&t) {
                            runtime.extend([EdgeLockedState(edge_sym, b)]);
                        }
                    }
                }
            }
            Requirement::AttrAtLeast { node, attr, threshold } => {
                let node_sym = sym.intern(&node);
                let attr_sym = sym.intern(&attr);
                runtime.extend([ReqAttrAtLeast(req_sym, node_sym, attr_sym, quantize(threshold))]);
                if seen_attrs.insert((node_sym, attr_sym)) {
                    if let Some(t) = graph.object(&node, &attr) {
                        if let Some(f) = as_float(&t) {
                            runtime.extend([AttrValue(node_sym, attr_sym, quantize(f))]);
                        }
                    }
                }
            }
        }
    }

    let (_met, _unmet, ready) = runtime.run();
    ready
        .into_iter()
        .map(|AllRequirementsMet(m)| sym.resolve(m))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{lit_bool, lit_float, Delta, WorldGraph};
    use crate::machine::{build_action_machine, requirement_met, Effect, Requirement};

    /// Real equivalence check: for every machine this builds, the Datalog
    /// path (`machines_ready`) must agree with the existing hand-rolled
    /// `requirement_met` evaluated across all of that machine's own
    /// requirements -- not a self-consistent toy, a cross-check against
    /// the function this is meant to eventually replace.
    #[test]
    fn agrees_with_requirement_met_across_all_three_kinds() {
        let mut graph = WorldGraph::new();
        crate::demiurge::bootstrap(&mut graph);
        let room_a = graph.fresh("room/");
        let room_b = graph.fresh("room/");
        let edge = graph.fresh("edge/");
        let hero = graph.fresh("char/");
        let strength = graph.fresh("attr/strength");

        graph
            .commit(
                "test",
                Delta::new()
                    .assert(edge.clone(), vocab::locked(), lit_bool(true))
                    .assert(strength.clone(), vocab::rdf_type(), vocab::class_attribute())
                    .assert(hero.clone(), strength.clone(), lit_float(3.0)),
            )
            .expect("fixture delta is always valid");

        let owner = graph.fresh("owner/");
        graph
            .commit(
                "test",
                Delta::new().assert(owner.clone(), vocab::rdf_type(), vocab::class_item()),
            )
            .expect("owner-typing delta is always valid");
        let (machine_ready, d1) = build_action_machine(
            &mut graph,
            &owner,
            "push",
            &[
                Requirement::PlayerInRoom { room: room_a.clone() },
                Requirement::AttrAtLeast { node: hero.clone(), attr: strength.clone(), threshold: 2.0 },
            ],
            &Effect::SetEdgeLocked { edge: edge.clone(), value: false },
        );
        graph.commit("test", d1).expect("machine delta is always valid");

        let (machine_blocked, d2) = build_action_machine(
            &mut graph,
            &owner,
            "shove",
            &[
                Requirement::PlayerInRoom { room: room_b.clone() }, // player is NOT here
                Requirement::EdgeLocked { edge: edge.clone(), equals: false }, // edge IS locked -> false
            ],
            &Effect::SetEdgeLocked { edge: edge.clone(), value: false },
        );
        graph.commit("test", d2).expect("machine delta is always valid");

        let (machine_edge_ok, d3) = build_action_machine(
            &mut graph,
            &owner,
            "unbar",
            &[Requirement::EdgeLocked { edge: edge.clone(), equals: true }],
            &Effect::SetEdgeLocked { edge: edge.clone(), value: false },
        );
        graph.commit("test", d3).expect("machine delta is always valid");

        let (machine_attr_short, d4) = build_action_machine(
            &mut graph,
            &owner,
            "heave",
            &[Requirement::AttrAtLeast { node: hero.clone(), attr: strength.clone(), threshold: 10.0 }],
            &Effect::SetEdgeLocked { edge: edge.clone(), value: false },
        );
        graph.commit("test", d4).expect("machine delta is always valid");

        let ready = machines_ready(&graph, &room_a);

        for (machine, expect_ready) in [
            (&machine_ready, true),
            (&machine_blocked, false),
            (&machine_edge_ok, true),
            (&machine_attr_short, false),
        ] {
            let reqs: Vec<Requirement> = graph
                .objects(machine, &vocab::has_requirement())
                .into_iter()
                .filter_map(|t| match t {
                    oxigraph::model::Term::NamedNode(n) => read_requirement(&graph, &n),
                    _ => None,
                })
                .collect();
            let hand_rolled = reqs.iter().all(|r| requirement_met(&graph, r, &room_a));
            let datalog = ready.contains(machine);
            assert_eq!(
                hand_rolled, expect_ready,
                "test fixture sanity: hand-rolled result didn't match what this test expected"
            );
            assert_eq!(
                datalog, hand_rolled,
                "Datalog and hand-rolled requirement_met disagree for a machine"
            );
        }
    }
}
