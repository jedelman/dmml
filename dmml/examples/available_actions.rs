//! "Can valid actions be computed automatically or do you have to do
//! them by hand every time?" (Jason, 2026-08-30) -- automatically, for
//! real, using `machine::may_fire`, which already existed for exactly
//! this question but had never been used to enumerate anything before
//! now (every prior example called `commit_fires_transition` on ONE
//! named transition it already knew it wanted to test).
//!
//! For a param-less transition, `may_fire` is a direct answer: build an
//! empty-params `EvalContext`, ask, done. For a parameterized transition
//! (`mix($sand_source, $water_source)`, `build($brick_source,
//! $mortar_source)`, ...), there's no single ctx to ask -- "can this
//! fire" depends on WHICH nodes get cited. The real, computable answer:
//! try every existing machine node against every param slot (a Cartesian
//! product, small at this world's scale -- 10 nodes, at most 2 params,
//! well under a hundred combinations) and keep every binding that
//! actually passes `may_fire`. That's not a heuristic or an
//! approximation -- it's the literal, exhaustive answer to "which
//! concrete actions are legal right now," computed the same way the
//! guard itself will be checked at firing time.
//!
//! Output: one JSON object per currently-legal (node, transition,
//! params) combination -- the exact shape `valar_operate_test.py`'s
//! `oneOf` schema branches are built from, replacing the hand-typed
//! `CATALOG` list with something computed fresh from live world state
//! every time this runs. This is "how we should be surfacing the world
//! for agents": the schema the agent sees is never wider than what's
//! actually true right now.

use dmml::interpret::Materialized;
use dmml::lower::{lower_commit, LoweredCommit};
use dmml::machine::{self, EvalContext, MachineBody};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct AvailableAction {
    node: String,
    transition: String,
    params: Option<HashMap<String, String>>,
}

fn seed_json() -> &'static str {
    r#"{
  "update": [
    {
      "machines": [
        {
          "node": "Valinor",
          "states": [{"ident": "unformed"}, {"ident": "hills"}, {"ident": "mountains"}],
          "transitions": [
            {"ident": "raise", "from": "unformed", "to": "hills"},
            {"ident": "uplift", "from": "hills", "to": "mountains"}
          ]
        },
        {
          "node": "Valinor/quarry",
          "states": [{"ident": "untouched"}, {"ident": "stone"}, {"ident": "sand"}, {"ident": "clay"}, {"ident": "brick"}],
          "transitions": [
            {"ident": "quarry", "from": "untouched", "to": "stone",
              "guards": [{"exists": {"anchor": {"kind": "node", "value": "Valinor"},
                "hops": [{"predicate": "state", "term": {"kind": "node", "value": "mountains"}}]}}]},
            {"ident": "grind", "from": "stone", "to": "sand"},
            {"ident": "wet", "from": "sand", "to": "clay"},
            {"ident": "fire", "from": "clay", "to": "brick"}
          ]
        },
        {
          "node": "Valinor/streambed",
          "states": [{"ident": "bare"}, {"ident": "sand"}],
          "transitions": [{"ident": "wash", "from": "bare", "to": "sand"}]
        },
        {
          "node": "Valinor/spring",
          "states": [{"ident": "dry"}, {"ident": "flowing"}],
          "transitions": [{"ident": "well_up", "from": "dry", "to": "flowing"}]
        },
        {
          "node": "Valinor/mortar",
          "states": [{"ident": "unmixed"}, {"ident": "mixed"}],
          "transitions": [
            {"ident": "mix", "params": ["sand_source", "water_source"], "from": "unmixed", "to": "mixed",
              "guards": [
                {"exists": {"anchor": {"kind": "param", "value": "sand_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "sand"}}]}},
                {"exists": {"anchor": {"kind": "param", "value": "water_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "flowing"}}]}}
              ]}
          ]
        },
        {
          "node": "Valinor/wall",
          "states": [{"ident": "unbuilt"}, {"ident": "built"}],
          "transitions": [
            {"ident": "build", "params": ["brick_source", "mortar_source"], "from": "unbuilt", "to": "built",
              "guards": [
                {"exists": {"anchor": {"kind": "param", "value": "brick_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "brick"}}]}},
                {"exists": {"anchor": {"kind": "param", "value": "mortar_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "mixed"}}]}}
              ]}
          ]
        },
        {
          "node": "Valinor/forest",
          "states": [{"ident": "full"}, {"ident": "thinned"}, {"ident": "depleted"}],
          "transitions": [
            {"ident": "gather", "from": "full", "to": "thinned"},
            {"ident": "overgather", "from": "thinned", "to": "depleted"}
          ]
        },
        {
          "node": "Valinor/carpentry",
          "states": [{"ident": "no_frame"}, {"ident": "framed"}],
          "transitions": [
            {"ident": "make_frame", "from": "no_frame", "to": "framed",
              "guards": [{"negated": true, "exists": {"anchor": {"kind": "node", "value": "Valinor/forest"},
                "hops": [{"predicate": "state", "term": {"kind": "node", "value": "depleted"}}]}}]}
          ]
        },
        {
          "node": "Valinor/roof",
          "states": [{"ident": "unroofed"}, {"ident": "roofed"}],
          "transitions": [
            {"ident": "add_roof", "params": ["wall_source", "frame_source"], "from": "unroofed", "to": "roofed",
              "guards": [
                {"exists": {"anchor": {"kind": "param", "value": "wall_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "built"}}]}},
                {"exists": {"anchor": {"kind": "param", "value": "frame_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "framed"}}]}}
              ]}
          ]
        },
        {
          "node": "Valinor/house",
          "states": [{"ident": "unbuilt"}, {"ident": "built"}],
          "transitions": [
            {"ident": "construct_house", "from": "unbuilt", "to": "built",
              "guards": [
                {"exists": {"anchor": {"kind": "node", "value": "Valinor/wall"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "built"}}]}},
                {"exists": {"anchor": {"kind": "node", "value": "Valinor/roof"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "roofed"}}]}}
              ]}
          ]
        }
      ],
      "commits": [
        {
          "verb": "mints",
          "declares": [{"kind": "attribute", "name": "state"}],
          "facts": [
            {"subject": "Valinor", "predicate": "state", "object": {"kind": "node", "value": "unformed"}},
            {"subject": "Valinor/quarry", "predicate": "state", "object": {"kind": "node", "value": "untouched"}},
            {"subject": "Valinor/streambed", "predicate": "state", "object": {"kind": "node", "value": "bare"}},
            {"subject": "Valinor/spring", "predicate": "state", "object": {"kind": "node", "value": "dry"}},
            {"subject": "Valinor/mortar", "predicate": "state", "object": {"kind": "node", "value": "unmixed"}},
            {"subject": "Valinor/wall", "predicate": "state", "object": {"kind": "node", "value": "unbuilt"}},
            {"subject": "Valinor/forest", "predicate": "state", "object": {"kind": "node", "value": "full"}},
            {"subject": "Valinor/carpentry", "predicate": "state", "object": {"kind": "node", "value": "no_frame"}},
            {"subject": "Valinor/roof", "predicate": "state", "object": {"kind": "node", "value": "unroofed"}},
            {"subject": "Valinor/house", "predicate": "state", "object": {"kind": "node", "value": "unbuilt"}}
          ]
        }
      ]
    }
  ]
}"#
}

/// Every real node this world knows about -- anything with a `state`
/// fact. The candidate pool for param bindings: a $param can only
/// meaningfully cite something that actually exists.
fn known_nodes(world: &Materialized) -> Vec<String> {
    let mut nodes: Vec<String> = world
        .iter()
        .filter(|(_, pred, _)| *pred == "state")
        .map(|(subj, _, _)| subj.to_string())
        .collect();
    nodes.sort();
    nodes.dedup();
    nodes
}

/// Cartesian product of `candidates` taken `n` at a time, one binding
/// (param_name -> node) per combination -- exhaustive, not sampled.
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

fn main() {
    let update = dmml::from_json::update_from_json(seed_json()).expect("seed JSON is valid DMML");
    let mut machines: HashMap<String, MachineBody> = HashMap::new();
    for batch in &update.batches {
        for m in &batch.machines {
            machines.insert(m.node.segments.join("/"), MachineBody { states: m.states.clone(), transitions: m.transitions.clone() });
        }
    }
    let history: Vec<LoweredCommit> = update.batches.iter().flat_map(|b| b.commits.iter().map(lower_commit)).collect();
    let world = Materialized::from_commits(&history);
    let nodes = known_nodes(&world);

    let mut available = Vec::new();

    for (node, body) in &machines {
        for decl in &body.transitions {
            if decl.params.is_empty() {
                let ctx = EvalContext { self_node: node.clone(), params: HashMap::new() };
                if machine::may_fire(body, &decl.ident, &ctx, &world) == Some(true) {
                    available.push(AvailableAction { node: node.clone(), transition: decl.ident.clone(), params: None });
                }
            } else {
                for binding in param_bindings(&decl.params, &nodes) {
                    let ctx = EvalContext { self_node: node.clone(), params: binding.clone() };
                    if machine::may_fire(body, &decl.ident, &ctx, &world) == Some(true) {
                        available.push(AvailableAction { node: node.clone(), transition: decl.ident.clone(), params: Some(binding) });
                    }
                }
            }
        }
    }

    available.sort_by(|a, b| (a.node.as_str(), a.transition.as_str()).cmp(&(b.node.as_str(), b.transition.as_str())));

    println!("{}", serde_json::to_string_pretty(&available).unwrap());
    eprintln!(
        "\n{} currently-legal action(s) out of {} declared transitions across {} machines, {} candidate nodes.",
        available.len(),
        machines.values().map(|b| b.transitions.len()).sum::<usize>(),
        machines.len(),
        nodes.len(),
    );
}
