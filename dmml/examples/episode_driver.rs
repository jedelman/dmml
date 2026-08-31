//! "let's run a larger scale test for world modeling" (Jason,
//! 2026-08-31) -- scaled along the axis Jason picked: not a bigger
//! world, not more models, but a real MULTI-STEP EPISODE. Every prior
//! operate-tier test (`valar_operate_test.py`, Round 5) was a single
//! pick against the seed snapshot. This is the same mechanism run
//! forward: at each turn, recompute the legal-action schema from
//! whatever the world ACTUALLY is right now (not the seed), let
//! something choose one, fire it for real, and feed the new state into
//! the next turn -- until the house is built, nothing is legal anymore,
//! or a turn cap is hit.
//!
//! The house-world seed (`available_actions.rs`, `operate_check.rs`) has
//! a real branch and a real trap built in, both already present in the
//! grammar, neither added for this file: `Valinor/forest`'s `gather`
//! (full->thinned) is safe, but `overgather` (thinned->depleted) is a
//! dead end -- `Valinor/carpentry`'s `make_frame` guard is `NOT
//! EXISTS(Valinor/forest --state--> depleted)`, and nothing in this
//! world regrows a forest. Fire `overgather` and `make_frame` becomes
//! permanently illegal, which permanently blocks `add_roof`, which
//! permanently blocks `construct_house`. A full correct playthrough
//! needs 14 firings across a real dependency DAG (`Valinor` -> `quarry`
//! chain + `streambed`/`spring` -> `mortar` + `quarry`'s brick -> `wall`;
//! `forest` -> `carpentry` -> `roof` with `wall`; `wall` + `roof` ->
//! `house`) with a genuine choice point (`mortar`'s `sand_source` can
//! legally bind to either `Valinor/quarry` after `grind` or
//! `Valinor/streambed` after `wash`, whichever reaches `sand` state
//! first) and one avoidable trap.
//!
//! This binary is the world engine only -- it never picks an action
//! itself. It speaks one JSON object per line on stdout/stdin so a
//! driver (a human, a script, a dispatched model's own loop) can play
//! it: each stdout line is either `{"turn": N, "legal_actions": [...],
//! "state": {...}}` (your move) or `{"episode_over": true, "reason":
//! ...}` (done). Feed one line of `{"node":..., "transition":...,
//! "params": {...}|null}` on stdin per turn to fire a choice; anything
//! not currently in `legal_actions` is rejected with a `fire_result`
//! explaining why, per the same real `dmml::machine::commit_fires_
//! transition` check every prior example in this thread used, and the
//! turn is NOT consumed -- try again.
//!
//! World state is carried forward as a flat `Vec<LoweredCommit>` folded
//! with `Materialized::from_commits` (plain last-write-wins), the exact
//! same primitive `operate_check.rs`/`available_actions.rs` already use
//! for a single turn -- nothing new about how a turn is checked, only
//! that this file loops it and keeps the state instead of exiting after
//! one pick.
//!
//! Run interactively: `cargo run -p dmml --example episode_driver`.
//! Driven by a script: pipe chosen-action JSON lines in, read state/
//! legal-action JSON lines out.

use dmml::from_json::update_from_json;
use dmml::interpret::Materialized;
use dmml::lower::{ConsumeRef, FactRef, LoweredCommit, StrongRef, Triple, TripleValue};
use dmml::machine::{self, EvalContext, MachineBody};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

const MAX_TURNS: usize = 25;
const GOAL_NODE: &str = "Valinor/house";
const GOAL_STATE: &str = "built";

fn house_world_machines_json() -> &'static str {
    r#"{"update": [{"machines": [
        {"node": "Valinor", "states": [{"ident": "unformed"}, {"ident": "hills"}, {"ident": "mountains"}],
          "transitions": [
            {"ident": "raise", "from": "unformed", "to": "hills"},
            {"ident": "uplift", "from": "hills", "to": "mountains"}]},
        {"node": "Valinor/quarry", "states": [{"ident": "untouched"}, {"ident": "stone"}, {"ident": "sand"}, {"ident": "clay"}, {"ident": "brick"}],
          "transitions": [
            {"ident": "quarry", "from": "untouched", "to": "stone",
              "guards": [{"exists": {"anchor": {"kind": "node", "value": "Valinor"},
                "hops": [{"predicate": "state", "term": {"kind": "node", "value": "mountains"}}]}}]},
            {"ident": "grind", "from": "stone", "to": "sand"},
            {"ident": "wet", "from": "sand", "to": "clay"},
            {"ident": "fire", "from": "clay", "to": "brick"}]},
        {"node": "Valinor/streambed", "states": [{"ident": "bare"}, {"ident": "sand"}],
          "transitions": [{"ident": "wash", "from": "bare", "to": "sand"}]},
        {"node": "Valinor/spring", "states": [{"ident": "dry"}, {"ident": "flowing"}],
          "transitions": [{"ident": "well_up", "from": "dry", "to": "flowing"}]},
        {"node": "Valinor/mortar", "states": [{"ident": "unmixed"}, {"ident": "mixed"}],
          "transitions": [
            {"ident": "mix", "params": ["sand_source", "water_source"], "from": "unmixed", "to": "mixed",
              "guards": [
                {"exists": {"anchor": {"kind": "param", "value": "sand_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "sand"}}]}},
                {"exists": {"anchor": {"kind": "param", "value": "water_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "flowing"}}]}}]}]},
        {"node": "Valinor/wall", "states": [{"ident": "unbuilt"}, {"ident": "built"}],
          "transitions": [
            {"ident": "build", "params": ["brick_source", "mortar_source"], "from": "unbuilt", "to": "built",
              "guards": [
                {"exists": {"anchor": {"kind": "param", "value": "brick_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "brick"}}]}},
                {"exists": {"anchor": {"kind": "param", "value": "mortar_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "mixed"}}]}}]}]},
        {"node": "Valinor/forest", "states": [{"ident": "full"}, {"ident": "thinned"}, {"ident": "depleted"}],
          "transitions": [
            {"ident": "gather", "from": "full", "to": "thinned"},
            {"ident": "overgather", "from": "thinned", "to": "depleted"}]},
        {"node": "Valinor/carpentry", "states": [{"ident": "no_frame"}, {"ident": "framed"}],
          "transitions": [
            {"ident": "make_frame", "from": "no_frame", "to": "framed",
              "guards": [{"negated": true, "exists": {"anchor": {"kind": "node", "value": "Valinor/forest"},
                "hops": [{"predicate": "state", "term": {"kind": "node", "value": "depleted"}}]}}]}]},
        {"node": "Valinor/roof", "states": [{"ident": "unroofed"}, {"ident": "roofed"}],
          "transitions": [
            {"ident": "add_roof", "params": ["wall_source", "frame_source"], "from": "unroofed", "to": "roofed",
              "guards": [
                {"exists": {"anchor": {"kind": "param", "value": "wall_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "built"}}]}},
                {"exists": {"anchor": {"kind": "param", "value": "frame_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "framed"}}]}}]}]},
        {"node": "Valinor/house", "states": [{"ident": "unbuilt"}, {"ident": "built"}],
          "transitions": [
            {"ident": "construct_house", "from": "unbuilt", "to": "built",
              "guards": [
                {"exists": {"anchor": {"kind": "node", "value": "Valinor/wall"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "built"}}]}},
                {"exists": {"anchor": {"kind": "node", "value": "Valinor/roof"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "roofed"}}]}}]}]}
    ]}]}"#
}

fn seed_state() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Valinor", "unformed"),
        ("Valinor/quarry", "untouched"),
        ("Valinor/streambed", "bare"),
        ("Valinor/spring", "dry"),
        ("Valinor/mortar", "unmixed"),
        ("Valinor/wall", "unbuilt"),
        ("Valinor/forest", "full"),
        ("Valinor/carpentry", "no_frame"),
        ("Valinor/roof", "unroofed"),
        ("Valinor/house", "unbuilt"),
    ]
}

#[derive(Serialize)]
struct AvailableAction {
    node: String,
    transition: String,
    params: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct Choice {
    node: String,
    transition: String,
    #[serde(default)]
    params: Option<HashMap<String, String>>,
}

fn known_nodes(world: &Materialized) -> Vec<String> {
    let mut nodes: Vec<String> = world.iter().filter(|(_, p, _)| *p == "state").map(|(s, _, _)| s.to_string()).collect();
    nodes.sort();
    nodes.dedup();
    nodes
}

fn param_bindings(param_names: &[String], candidates: &[String]) -> Vec<HashMap<String, String>> {
    if param_names.is_empty() {
        return vec![HashMap::new()];
    }
    let mut results = vec![HashMap::new()];
    for name in param_names {
        let mut next = Vec::new();
        for existing in &results {
            for candidate in candidates {
                let mut binding = existing.clone();
                binding.insert(name.clone(), candidate.clone());
                next.push(binding);
            }
        }
        results = next;
    }
    results
}

fn legal_actions(machines: &HashMap<String, MachineBody>, world: &Materialized) -> Vec<AvailableAction> {
    let nodes = known_nodes(world);
    let mut available = Vec::new();
    for (node, body) in machines {
        for decl in &body.transitions {
            if decl.params.is_empty() {
                let ctx = EvalContext { self_node: node.clone(), params: HashMap::new() };
                if machine::may_fire(body, &decl.ident, &ctx, world) == Some(true) {
                    available.push(AvailableAction { node: node.clone(), transition: decl.ident.clone(), params: None });
                }
            } else {
                for binding in param_bindings(&decl.params, &nodes) {
                    let ctx = EvalContext { self_node: node.clone(), params: binding.clone() };
                    if machine::may_fire(body, &decl.ident, &ctx, world) == Some(true) {
                        available.push(AvailableAction { node: node.clone(), transition: decl.ident.clone(), params: Some(binding) });
                    }
                }
            }
        }
    }
    available.sort_by(|a, b| (a.node.as_str(), a.transition.as_str()).cmp(&(b.node.as_str(), b.transition.as_str())));
    available
}

fn state_snapshot(world: &Materialized) -> HashMap<String, String> {
    world
        .iter()
        .filter(|(_, p, _)| *p == "state")
        .filter_map(|(s, _, v)| match v {
            TripleValue::Node(n) => Some((s.to_string(), n.clone())),
            _ => None,
        })
        .collect()
}

/// Builds the candidate commit a chosen (node, transition, params) needs
/// directly as Rust structs -- no JSON round trip, since `LoweredCommit`/
/// `Triple`/`ConsumeRef` are all plain public-field structs. A dummy,
/// never-resolved `StrongRef` stands in for provenance: `commit_fires_
/// transition` only ever inspects a candidate's own `consumes`/`produces`
/// structurally (see its own doc comment on why a `Strong` consume is
/// never trusted unconditionally either) -- it never looks the reference
/// up against a real commit history, and this file's running world state
/// is a plain `Materialized::from_commits` last-write-wins fold, not the
/// `consumes`-driven retraction fold, so nothing downstream needs this
/// reference to resolve to anything real.
fn build_candidate(decl: &dmml::machine::TransitionDecl, node: &str, params: &HashMap<String, String>) -> LoweredCommit {
    let dummy_ref = StrongRef { uri: "at://did:example:episode/world.episode/turn".to_string(), cid: "turn-cid".to_string() };
    let mut produces = Vec::new();
    let mut consumes = Vec::new();

    if let (Some(from), Some(to)) = (&decl.from, &decl.to) {
        consumes.push(ConsumeRef::Fact(FactRef {
            commit: dummy_ref.clone(),
            subject: node.to_string(),
            predicate: "state".to_string(),
            object: Some(TripleValue::Node(from.clone())),
        }));
        produces.push(Triple { subject: node.to_string(), predicate: "state".to_string(), object: TripleValue::Node(to.clone()) });
    }
    for (name, value) in params {
        produces.push(Triple { subject: node.to_string(), predicate: name.clone(), object: TripleValue::Node(value.clone()) });
    }

    LoweredCommit { predicate_verb: decl.ident.clone(), consumes, produces, refs: HashMap::new() }
}

fn print_line<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string(value).unwrap());
    io::stdout().flush().unwrap();
}

fn main() {
    let update = update_from_json(house_world_machines_json()).expect("machine defs are valid DMML");
    let mut machines: HashMap<String, MachineBody> = HashMap::new();
    for batch in &update.batches {
        for m in &batch.machines {
            machines.insert(m.node.segments.join("/"), MachineBody { states: m.states.clone(), transitions: m.transitions.clone() });
        }
    }

    let mut history: Vec<LoweredCommit> = vec![Triple {
        subject: "seed".to_string(),
        predicate: "unused".to_string(),
        object: TripleValue::Boolean(true),
    }]
    .into_iter()
    .map(|_| LoweredCommit {
        predicate_verb: "mints".to_string(),
        consumes: Vec::new(),
        produces: seed_state().into_iter().map(|(n, s)| Triple { subject: n.to_string(), predicate: "state".to_string(), object: TripleValue::Node(s.to_string()) }).collect(),
        refs: HashMap::new(),
    })
    .collect();

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    for turn in 1..=MAX_TURNS {
        let world = Materialized::from_commits(&history);

        if world.current_value(GOAL_NODE, "state") == Some(&TripleValue::Node(GOAL_STATE.to_string())) {
            print_line(&serde_json::json!({"episode_over": true, "reason": "goal_reached", "turns_taken": turn - 1, "state": state_snapshot(&world)}));
            return;
        }

        let actions = legal_actions(&machines, &world);
        if actions.is_empty() {
            print_line(&serde_json::json!({"episode_over": true, "reason": "no_legal_actions", "turns_taken": turn - 1, "state": state_snapshot(&world)}));
            return;
        }

        print_line(&serde_json::json!({"turn": turn, "legal_actions": actions, "state": state_snapshot(&world)}));

        let Some(Ok(line)) = lines.next() else {
            print_line(&serde_json::json!({"episode_over": true, "reason": "stdin_closed", "turns_taken": turn - 1}));
            return;
        };
        let choice: Choice = match serde_json::from_str(&line) {
            Ok(c) => c,
            Err(e) => {
                print_line(&serde_json::json!({"turn": turn, "fire_result": format!("FAIL: could not parse choice: {e}"), "state": state_snapshot(&world)}));
                continue;
            }
        };

        let Some(body) = machines.get(&choice.node) else {
            print_line(&serde_json::json!({"turn": turn, "fire_result": format!("FAIL: no machine named '{}'", choice.node), "state": state_snapshot(&world)}));
            continue;
        };
        let Some(decl) = body.transitions.iter().find(|t| t.ident == choice.transition) else {
            print_line(&serde_json::json!({"turn": turn, "fire_result": format!("FAIL: '{}' has no transition '{}'", choice.node, choice.transition), "state": state_snapshot(&world)}));
            continue;
        };
        let params = choice.params.unwrap_or_default();
        let candidate = build_candidate(decl, &choice.node, &params);
        let ctx = EvalContext { self_node: choice.node.clone(), params: params.clone() };

        match machine::commit_fires_transition(body, &choice.transition, &ctx, &world, &candidate) {
            Ok(()) => {
                history.push(candidate);
                print_line(&serde_json::json!({"turn": turn, "fire_result": "PASS", "fired": {"node": choice.node, "transition": choice.transition, "params": params}}));
            }
            Err(e) => {
                print_line(&serde_json::json!({"turn": turn, "fire_result": format!("FAIL: {e:?}"), "state": state_snapshot(&world)}));
            }
        }
    }

    let world = Materialized::from_commits(&history);
    print_line(&serde_json::json!({"episode_over": true, "reason": "turn_cap_reached", "turns_taken": MAX_TURNS, "state": state_snapshot(&world)}));
}
