//! One-shot success. "Can we tighten our tool schema to be enough for
//! the model to one shot it? all constraints should be structural"
//! (Jason, 2026-08-30) -- and it was. The exact same model
//! (deepseek-v4-flash-0731) at the exact same low reasoning effort had
//! failed the Vala design task 4/4 times running (`valar_mint.py`/
//! `valar_mint_loop.py`): `has_content` ("a transition needs a guard, a
//! from+to pair, or an effect") lived only as prose, the schema let a
//! transition satisfying none of the three validate anyway, and even a
//! real 5-round feedback loop with the exact validator error fed back
//! every round never converged -- rounds 2 and 3 even produced
//! byte-for-byte identical broken JSON despite explicit correct
//! reasoning each time.
//!
//! `valar_mint_strict.py` made the constraint structural instead:
//! `TransitionInput` became an `anyOf` of three required-shaped
//! branches (guard-bearing, from+to-bearing, effect-bearing), `strict:
//! true` (real constrained decoding, confirmed supported for this
//! model), `additionalProperties: false` and nullable-but-required
//! fields throughout, and the schema narrowed to only what the Vala
//! needs. First attempt, no loop, no retry: `Valinor/house`, gated on
//! TWO real cross-node guards (`Valinor/wall` must be `built`,
//! `Valinor/roof` must be `roofed`) -- correctly using the `state`
//! predicate (no `a`/rdf:type confusion, despite that ambiguity sinking
//! an earlier gpt-5.2-pro attempt), correctly citing fixed node anchors,
//! genuinely completing the production chain `house.rs` builds toward
//! but never gave its own dedicated machine.
//!
//! This file wires that one-shot design into `house.rs`'s world and
//! fires it for real: build the whole chain up through a roofed house,
//! then construct `Valinor/house` itself, plus a negative control
//! (constructing before the roof is on) correctly blocked.

use dmml::from_json::update_from_json;
use dmml::interpret::Materialized;
use dmml::lower::{lower_commit, LoweredCommit, TripleValue};
use dmml::machine::{self, EvalContext, MachineBody};
use std::collections::HashMap;

const SEED_URI: &str = "at://did:example:valinor/world.valinor/house-machine-genesis";
const SEED_CID: &str = "house-machine-seed-cid-0";

fn seed_json() -> String {
    format!(
        r#"{{
  "update": [
    {{
      "machines": [
        {{
          "node": "Valinor",
          "states": [{{"ident": "unformed"}}, {{"ident": "hills"}}, {{"ident": "mountains"}}],
          "transitions": [
            {{"ident": "raise", "from": "unformed", "to": "hills"}},
            {{"ident": "uplift", "from": "hills", "to": "mountains"}}
          ]
        }},
        {{
          "node": "Valinor/quarry",
          "states": [{{"ident": "untouched"}}, {{"ident": "stone"}}, {{"ident": "sand"}}, {{"ident": "clay"}}, {{"ident": "brick"}}],
          "transitions": [
            {{"ident": "quarry", "from": "untouched", "to": "stone",
              "guards": [{{"exists": {{"anchor": {{"kind": "node", "value": "Valinor"}},
                "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "mountains"}}}}]}}}}]}},
            {{"ident": "grind", "from": "stone", "to": "sand"}},
            {{"ident": "wet", "from": "sand", "to": "clay"}},
            {{"ident": "fire", "from": "clay", "to": "brick"}}
          ]
        }},
        {{
          "node": "Valinor/streambed",
          "states": [{{"ident": "bare"}}, {{"ident": "sand"}}],
          "transitions": [{{"ident": "wash", "from": "bare", "to": "sand"}}]
        }},
        {{
          "node": "Valinor/spring",
          "states": [{{"ident": "dry"}}, {{"ident": "flowing"}}],
          "transitions": [{{"ident": "well_up", "from": "dry", "to": "flowing"}}]
        }},
        {{
          "node": "Valinor/mortar",
          "states": [{{"ident": "unmixed"}}, {{"ident": "mixed"}}],
          "transitions": [
            {{"ident": "mix", "params": ["sand_source", "water_source"], "from": "unmixed", "to": "mixed",
              "guards": [
                {{"exists": {{"anchor": {{"kind": "param", "value": "sand_source"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "sand"}}}}]}}}},
                {{"exists": {{"anchor": {{"kind": "param", "value": "water_source"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "flowing"}}}}]}}}}
              ]}}
          ]
        }},
        {{
          "node": "Valinor/wall",
          "states": [{{"ident": "unbuilt"}}, {{"ident": "built"}}],
          "transitions": [
            {{"ident": "build", "params": ["brick_source", "mortar_source"], "from": "unbuilt", "to": "built",
              "guards": [
                {{"exists": {{"anchor": {{"kind": "param", "value": "brick_source"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "brick"}}}}]}}}},
                {{"exists": {{"anchor": {{"kind": "param", "value": "mortar_source"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "mixed"}}}}]}}}}
              ]}}
          ]
        }},
        {{
          "node": "Valinor/forest",
          "states": [{{"ident": "full"}}, {{"ident": "thinned"}}, {{"ident": "depleted"}}],
          "transitions": [
            {{"ident": "gather", "from": "full", "to": "thinned"}},
            {{"ident": "overgather", "from": "thinned", "to": "depleted"}}
          ]
        }},
        {{
          "node": "Valinor/carpentry",
          "states": [{{"ident": "no_frame"}}, {{"ident": "framed"}}],
          "transitions": [
            {{"ident": "make_frame", "from": "no_frame", "to": "framed",
              "guards": [{{"negated": true, "exists": {{"anchor": {{"kind": "node", "value": "Valinor/forest"}},
                "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "depleted"}}}}]}}}}]}}
          ]
        }},
        {{
          "node": "Valinor/roof",
          "states": [{{"ident": "unroofed"}}, {{"ident": "roofed"}}],
          "transitions": [
            {{"ident": "add_roof", "params": ["wall_source", "frame_source"], "from": "unroofed", "to": "roofed",
              "guards": [
                {{"exists": {{"anchor": {{"kind": "param", "value": "wall_source"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "built"}}}}]}}}},
                {{"exists": {{"anchor": {{"kind": "param", "value": "frame_source"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "framed"}}}}]}}}}
              ]}}
          ]
        }},
        {{
          "node": "Valinor/house",
          "states": [{{"ident": "unbuilt"}}, {{"ident": "built"}}],
          "transitions": [
            {{"ident": "construct_house", "params": [], "from": "unbuilt", "to": "built",
              "guards": [
                {{"negated": false, "exists": {{"anchor": {{"kind": "node", "value": "Valinor/wall"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "built"}}}}]}}}},
                {{"negated": false, "exists": {{"anchor": {{"kind": "node", "value": "Valinor/roof"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "roofed"}}}}]}}}}
              ]}}
          ]
        }}
      ],
      "commits": [
        {{
          "verb": "mints",
          "declares": [{{"kind": "attribute", "name": "state"}}],
          "facts": [
            {{"subject": "Valinor", "predicate": "state", "object": {{"kind": "node", "value": "unformed"}}}},
            {{"subject": "Valinor/quarry", "predicate": "state", "object": {{"kind": "node", "value": "untouched"}}}},
            {{"subject": "Valinor/streambed", "predicate": "state", "object": {{"kind": "node", "value": "bare"}}}},
            {{"subject": "Valinor/spring", "predicate": "state", "object": {{"kind": "node", "value": "dry"}}}},
            {{"subject": "Valinor/mortar", "predicate": "state", "object": {{"kind": "node", "value": "unmixed"}}}},
            {{"subject": "Valinor/wall", "predicate": "state", "object": {{"kind": "node", "value": "unbuilt"}}}},
            {{"subject": "Valinor/forest", "predicate": "state", "object": {{"kind": "node", "value": "full"}}}},
            {{"subject": "Valinor/carpentry", "predicate": "state", "object": {{"kind": "node", "value": "no_frame"}}}},
            {{"subject": "Valinor/roof", "predicate": "state", "object": {{"kind": "node", "value": "unroofed"}}}},
            {{"subject": "Valinor/house", "predicate": "state", "object": {{"kind": "node", "value": "unbuilt"}}}}
          ]
        }}
      ]
    }}
  ]
}}"#
    )
}

fn fire_json(node: &str, transition: &str, from_state: &str, to_state: &str, extra: &str, declares: &str) -> String {
    format!(
        r#"{{
  "update": [{{"commits": [{{
    "verb": "{transition}",
    "declares": [{declares}],
    "consumes": [
      {{"kind": "fact", "commit": {{"uri": "{SEED_URI}", "cid": "{SEED_CID}"}},
       "subject": "{node}", "predicate": "state", "object": {{"kind": "node", "value": "{from_state}"}}}}
    ],
    "facts": [
      {{"subject": "{node}", "predicate": "state", "object": {{"kind": "node", "value": "{to_state}"}}}}
      {extra}
    ]
  }}]}}]
}}"#
    )
}

fn extract_param(commit: &LoweredCommit, self_node: &str, param_name: &str) -> Option<String> {
    commit.produces.iter().find_map(|t| {
        if t.subject == self_node && t.predicate == param_name {
            if let TripleValue::Node(v) = &t.object {
                return Some(v.clone());
            }
        }
        None
    })
}

fn machines_from_batches(update: &dmml::from_json::Update) -> HashMap<String, MachineBody> {
    let mut map = HashMap::new();
    for batch in &update.batches {
        for m in &batch.machines {
            map.insert(m.node.segments.join("/"), MachineBody { states: m.states.clone(), transitions: m.transitions.clone() });
        }
    }
    map
}

fn print_world(world: &Materialized) {
    let mut rows: Vec<(&str, &str, &TripleValue)> = world.iter().collect();
    rows.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    for (subj, pred, val) in rows {
        println!("  {subj:16} {pred:12} {val:?}");
    }
}

fn try_fire(
    label: &str,
    machines: &HashMap<String, MachineBody>,
    world_before: &Materialized,
    node: &str,
    transition: &str,
    params: HashMap<String, String>,
    candidate: &LoweredCommit,
) -> bool {
    let body = machines.get(node).expect("machine declared");
    let ctx = EvalContext { self_node: node.to_string(), params };
    match machine::commit_fires_transition(body, transition, &ctx, world_before, candidate) {
        Ok(()) => {
            println!("  [OK]      {label}: '{transition}' on {node} fired legitimately.");
            true
        }
        Err(e) => {
            println!("  [BLOCKED] {label}: '{transition}' on {node} rejected: {e:?}");
            false
        }
    }
}

fn fire_simple(
    history: &mut Vec<LoweredCommit>,
    machines: &HashMap<String, MachineBody>,
    label: &str,
    node: &str,
    transition: &str,
    from_s: &str,
    to_s: &str,
) {
    let json = fire_json(node, transition, from_s, to_s, "", "");
    let update = update_from_json(&json).expect("valid DMML");
    let commit = lower_commit(&update.batches[0].commits[0]);
    let world_before = Materialized::from_commits(history);
    let fired = try_fire(label, machines, &world_before, node, transition, HashMap::new(), &commit);
    assert!(fired, "{label} ({transition} on {node}) should succeed");
    history.push(commit);
}

fn fire_two_param(
    history: &mut Vec<LoweredCommit>,
    machines: &HashMap<String, MachineBody>,
    label: &str,
    node: &str,
    transition: &str,
    from_s: &str,
    to_s: &str,
    a_name: &str,
    a_value: &str,
    b_name: &str,
    b_value: &str,
) {
    let extra = format!(
        r#", {{"subject": "{node}", "predicate": "{a_name}", "object": {{"kind": "node", "value": "{a_value}"}}}}, {{"subject": "{node}", "predicate": "{b_name}", "object": {{"kind": "node", "value": "{b_value}"}}}}"#
    );
    let declares = format!(r#"{{"kind": "attribute", "name": "{a_name}"}}, {{"kind": "attribute", "name": "{b_name}"}}"#);
    let json = fire_json(node, transition, from_s, to_s, &extra, &declares);
    let update = update_from_json(&json).expect("valid DMML");
    let commit = lower_commit(&update.batches[0].commits[0]);
    let a = extract_param(&commit, node, a_name).unwrap();
    let b = extract_param(&commit, node, b_name).unwrap();
    let world_before = Materialized::from_commits(history);
    let fired = try_fire(label, machines, &world_before, node, transition,
        HashMap::from([(a_name.to_string(), a), (b_name.to_string(), b)]), &commit);
    assert!(fired, "{label} ({transition} on {node}) should succeed");
    history.push(commit);
}

fn main() {
    let seed_update = update_from_json(&seed_json()).expect("seed JSON is valid DMML");
    let machines = machines_from_batches(&seed_update);
    let mut history: Vec<LoweredCommit> = seed_update.batches.iter().flat_map(|b| b.commits.iter().map(lower_commit)).collect();

    println!("########## Seed ##########\n");
    print_world(&Materialized::from_commits(&history));

    println!("\n########## Build the whole chain up through a roofed house ##########\n");
    fire_simple(&mut history, &machines, "raise", "Valinor", "raise", "unformed", "hills");
    fire_simple(&mut history, &machines, "uplift", "Valinor", "uplift", "hills", "mountains");
    fire_simple(&mut history, &machines, "quarry", "Valinor/quarry", "quarry", "untouched", "stone");
    fire_simple(&mut history, &machines, "grind", "Valinor/quarry", "grind", "stone", "sand");
    fire_simple(&mut history, &machines, "wet", "Valinor/quarry", "wet", "sand", "clay");
    fire_simple(&mut history, &machines, "fire", "Valinor/quarry", "fire", "clay", "brick");
    fire_simple(&mut history, &machines, "wash", "Valinor/streambed", "wash", "bare", "sand");
    fire_simple(&mut history, &machines, "well_up", "Valinor/spring", "well_up", "dry", "flowing");
    fire_two_param(&mut history, &machines, "mix", "Valinor/mortar", "mix", "unmixed", "mixed",
        "sand_source", "Valinor/streambed", "water_source", "Valinor/spring");
    fire_two_param(&mut history, &machines, "build", "Valinor/wall", "build", "unbuilt", "built",
        "brick_source", "Valinor/quarry", "mortar_source", "Valinor/mortar");
    fire_simple(&mut history, &machines, "make_frame", "Valinor/carpentry", "make_frame", "no_frame", "framed");

    println!("\n########## Negative control: construct the house before the roof is on ##########\n");
    let early_json = fire_json("Valinor/house", "construct_house", "unbuilt", "built", "", "");
    let early_update = update_from_json(&early_json).expect("valid DMML");
    let early_commit = lower_commit(&early_update.batches[0].commits[0]);
    let world_now = Materialized::from_commits(&history);
    let fired_early = try_fire("negative control", &machines, &world_now, "Valinor/house", "construct_house", HashMap::new(), &early_commit);
    assert!(!fired_early, "constructing the house before the roof is on must be blocked");

    fire_two_param(&mut history, &machines, "add_roof", "Valinor/roof", "add_roof", "unroofed", "roofed",
        "wall_source", "Valinor/wall", "frame_source", "Valinor/carpentry");

    println!("\n########## Construct the house -- the Vala's own one-shot design, now legitimate ##########\n");
    fire_simple(&mut history, &machines, "construct_house", "Valinor/house", "construct_house", "unbuilt", "built");

    println!("\n########## Final world state ##########\n");
    let final_world = Materialized::from_commits(&history);
    print_world(&final_world);
    println!("\n{} distinct (subject, predicate) triples across {} commits.", final_world.len(), history.len());
    println!("\nA one-shot Vala design, never touched by hand, fires cleanly end to end.");
}
