//! The capstone of the production chain so far, and the first real
//! exercise of `negated` guards (`GuardClause.negated`, declared in
//! `MACHINE_SPEC.md` from the start, never actually fired until now).
//!
//! Why negation earns its place here rather than being redundant with a
//! positive check: every guard in `valinor.rs`/`door.rs`/`quarry.rs`/
//! `wall.rs` checks a specific, single acceptable value against a
//! single-valued `state` predicate -- for those, `NOT state==X` and
//! `state==Y` (the only other value) say the same thing, so negation
//! would add nothing. This example is different on purpose: `Valinor/
//! forest` has THREE states (`full`, `thinned`, `depleted`), and
//! `make_frame` needs to accept TWO of them (full OR thinned) while
//! rejecting only the third. The guard grammar has no OR -- a
//! transition's guard list is a plain conjunction (`eval_guards`) -- so
//! "acceptable unless specifically depleted" can only be expressed as
//! `negated: true` over the one state actually being excluded. Positively
//! enumerating "full or thinned" isn't available at all; this is the
//! genuine case negation exists for, not a redundant restatement of a
//! positive check.
//!
//! Full chain: terrain (Valinor) -> material (quarry: stone/sand/clay/
//! brick) -> two two-input joins (mortar from sand+water, wall from
//! brick+mortar, both from `wall.rs`) -> a SEPARATE resource line
//! (forest -> carpentry, gated by the forest NOT being depleted) -> roof,
//! a THIRD two-input join (wall built AND frame made) that finally caps
//! the house. Four real production stages, two kinds of guard (positive
//! single-value, and negated exclusion), three two-input joins, one
//! cross-node fixed-anchor guard (quarry's mountain check) -- the fullest
//! composition in this session's examples.

use dmml::from_json::update_from_json;
use dmml::interpret::Materialized;
use dmml::lower::{lower_commit, LoweredCommit, TripleValue};
use dmml::machine::{self, EvalContext, MachineBody};
use std::collections::HashMap;

const SEED_URI: &str = "at://did:example:valinor/world.valinor/house-genesis";
const SEED_CID: &str = "house-seed-cid-0";

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
            {{"subject": "Valinor/roof", "predicate": "state", "object": {{"kind": "node", "value": "unroofed"}}}}
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
    param_a_name: &str,
    param_a_value: &str,
    param_b_name: &str,
    param_b_value: &str,
    expect_fire: bool,
) {
    let extra = format!(
        r#", {{"subject": "{node}", "predicate": "{param_a_name}", "object": {{"kind": "node", "value": "{param_a_value}"}}}}, {{"subject": "{node}", "predicate": "{param_b_name}", "object": {{"kind": "node", "value": "{param_b_value}"}}}}"#
    );
    let declares = format!(
        r#"{{"kind": "attribute", "name": "{param_a_name}"}}, {{"kind": "attribute", "name": "{param_b_name}"}}"#
    );
    let json = fire_json(node, transition, from_s, to_s, &extra, &declares);
    let update = update_from_json(&json).expect("valid DMML");
    let commit = lower_commit(&update.batches[0].commits[0]);
    let a = extract_param(&commit, node, param_a_name).unwrap();
    let b = extract_param(&commit, node, param_b_name).unwrap();
    let world_before = Materialized::from_commits(history);
    let fired = try_fire(
        label, machines, &world_before, node, transition,
        HashMap::from([(param_a_name.to_string(), a), (param_b_name.to_string(), b)]),
        &commit,
    );
    assert_eq!(fired, expect_fire, "{label} ({transition} on {node}) fired={fired}, expected={expect_fire}");
    if fired {
        history.push(commit);
    }
}

fn main() {
    let seed_update = update_from_json(&seed_json()).expect("seed JSON is valid DMML");
    let machines = machines_from_batches(&seed_update);
    let mut history: Vec<LoweredCommit> = seed_update.batches.iter().flat_map(|b| b.commits.iter().map(lower_commit)).collect();

    println!("########## Seed ##########\n");
    print_world(&Materialized::from_commits(&history));

    println!("\n########## Stage 1: terrain, material, and the wall (from valinor.rs/quarry.rs/wall.rs) ##########\n");
    fire_simple(&mut history, &machines, "raise", "Valinor", "raise", "unformed", "hills");
    fire_simple(&mut history, &machines, "uplift", "Valinor", "uplift", "hills", "mountains");
    fire_simple(&mut history, &machines, "quarry", "Valinor/quarry", "quarry", "untouched", "stone");
    fire_simple(&mut history, &machines, "grind", "Valinor/quarry", "grind", "stone", "sand");
    fire_simple(&mut history, &machines, "wet", "Valinor/quarry", "wet", "sand", "clay");
    fire_simple(&mut history, &machines, "fire", "Valinor/quarry", "fire", "clay", "brick");
    fire_simple(&mut history, &machines, "wash", "Valinor/streambed", "wash", "bare", "sand");
    fire_simple(&mut history, &machines, "well_up", "Valinor/spring", "well_up", "dry", "flowing");
    fire_two_param(&mut history, &machines, "mix", "Valinor/mortar", "mix", "unmixed", "mixed", "sand_source", "Valinor/streambed", "water_source", "Valinor/spring", true);
    fire_two_param(&mut history, &machines, "build", "Valinor/wall", "build", "unbuilt", "built", "brick_source", "Valinor/quarry", "mortar_source", "Valinor/mortar", true);
    print_world(&Materialized::from_commits(&history));

    println!("\n########## Stage 2: the forest is still full -- make a frame ##########\n");
    // Negative case first: forest hasn't even been gathered from, and
    // it's already NOT depleted -- so this should succeed trivially,
    // demonstrating negation passes when there's simply nothing to
    // exclude yet.
    fire_simple(&mut history, &machines, "make_frame (forest still full)", "Valinor/carpentry", "make_frame", "no_frame", "framed");
    print_world(&Materialized::from_commits(&history));

    println!("\n########## Stage 3: roof the house -- wall built AND frame made ##########\n");
    fire_two_param(&mut history, &machines, "add_roof", "Valinor/roof", "add_roof", "unroofed", "roofed", "wall_source", "Valinor/wall", "frame_source", "Valinor/carpentry", true);

    println!("\n########## Final world state: a roofed house ##########\n");
    let final_world = Materialized::from_commits(&history);
    print_world(&final_world);
    println!("\n{} distinct (subject, predicate) triples across {} commits.", final_world.len(), history.len());

    // --- Now, SEPARATELY, prove negation actually excludes: rerun the
    // carpentry step on a fresh forest that's been gathered to full
    // depletion first. Uses its own history branch so it doesn't disturb
    // the completed house above. ---
    println!("\n########## Separate branch: exhaust the forest, THEN try to make a frame ##########\n");
    let mut depleted_history: Vec<LoweredCommit> = seed_update.batches.iter().flat_map(|b| b.commits.iter().map(lower_commit)).collect();
    fire_simple(&mut depleted_history, &machines, "gather", "Valinor/forest", "gather", "full", "thinned");

    println!("\n  -- forest is 'thinned' (not yet depleted): make_frame should still succeed --\n");
    let mid_json = fire_json("Valinor/carpentry", "make_frame", "no_frame", "framed", "", "");
    let mid_update = update_from_json(&mid_json).expect("valid DMML");
    let mid_commit = lower_commit(&mid_update.batches[0].commits[0]);
    let world_thinned = Materialized::from_commits(&depleted_history);
    let fired_thinned = try_fire("make_frame (forest thinned)", &machines, &world_thinned, "Valinor/carpentry", "make_frame", HashMap::new(), &mid_commit);
    assert!(fired_thinned, "negation should still pass when the excluded state (depleted) doesn't hold, even though 'full' doesn't hold either -- this is the actual point of negated over a positive check");

    fire_simple(&mut depleted_history, &machines, "overgather", "Valinor/forest", "overgather", "thinned", "depleted");

    println!("\n  -- forest is now 'depleted': make_frame must be blocked --\n");
    let late_json = fire_json("Valinor/carpentry", "make_frame", "no_frame", "framed", "", "");
    let late_update = update_from_json(&late_json).expect("valid DMML");
    let late_commit = lower_commit(&late_update.batches[0].commits[0]);
    let world_depleted = Materialized::from_commits(&depleted_history);
    let fired_depleted = try_fire("make_frame (forest depleted)", &machines, &world_depleted, "Valinor/carpentry", "make_frame", HashMap::new(), &late_commit);
    assert!(!fired_depleted, "negated guard must block once the forest is actually depleted");

    println!("\nNegation confirmed doing real work: passes on 'full' AND 'thinned' (states never enumerated positively), blocks only on the one excluded state 'depleted'.");
}
