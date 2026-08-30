//! "Machines that consume" (Jason, 2026-08-30): the prior two examples
//! (`valinor.rs`, `door.rs`) mostly cycle ONE machine's own state, guarded
//! by its own prior fact or a cited target's fact. This one chains THREE
//! machines through real cross-node dependency: terrain gates material,
//! material differentiation gates minting a room. Nothing here is
//! narrated into existence -- every step is a guard checked against a
//! DIFFERENT node's live state, not self's own.
//!
//! Three machines, one dependency chain:
//!
//!   `Valinor` (terrain, same sculpting machine as valinor.rs):
//!     unformed -> hills -> mountains
//!
//!   `Valinor/quarry` (material differentiation -- real production, one
//!   state consumed to yield the next, echoing this project's own
//!   desiring-production framing more literally than any prior example):
//!     untouched -> stone -> sand -> clay -> brick
//!     `quarry` (untouched -> stone) is gated on Valinor itself being
//!     `mountains` -- the FIRST guard in this codebase's own examples
//!     anchored on a fixed OTHER node rather than self or a $param.
//!
//!   `Valinor/hall` (a room, minted only once the material is ready):
//!     unmined -> carved
//!     `carve($material_source)` is gated on the cited quarry being
//!     specifically `stone` -- you cannot carve a hall out of sand, clay,
//!     or untouched ground, only stone. This is "minting a room" made
//!     real: it consumes a material fact, not narrative permission.
//!
//! The full chain, checked end to end: raise the land, uplift it into
//! mountains, quarry stone from the mountain, THEN carve a hall from
//! that stone. Every illegitimate shortcut (quarrying before there's a
//! mountain, carving before there's stone) is run as a negative control
//! and confirmed blocked.

use dmml::from_json::update_from_json;
use dmml::interpret::Materialized;
use dmml::lower::{lower_commit, LoweredCommit, TripleValue};
use dmml::machine::{self, EvalContext, MachineBody};
use std::collections::HashMap;

const SEED_URI: &str = "at://did:example:valinor/world.valinor/quarry-genesis";
const SEED_CID: &str = "quarry-seed-cid-0";

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
          "node": "Valinor/hall",
          "states": [{{"ident": "unmined"}}, {{"ident": "carved"}}],
          "transitions": [
            {{"ident": "carve", "params": ["material_source"], "from": "unmined", "to": "carved",
              "guards": [{{"exists": {{"anchor": {{"kind": "param", "value": "material_source"}},
                "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "stone"}}}}]}}}}]}}
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
            {{"subject": "Valinor/hall", "predicate": "state", "object": {{"kind": "node", "value": "unmined"}}}}
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

/// Returns whether it fired -- callers use this both to assert success
/// on the legitimate path and to assert failure on negative controls.
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

fn main() {
    let seed_update = update_from_json(&seed_json()).expect("seed JSON is valid DMML");
    let machines = machines_from_batches(&seed_update);
    let seed_commits: Vec<LoweredCommit> = seed_update.batches.iter().flat_map(|b| b.commits.iter().map(lower_commit)).collect();
    let mut history = seed_commits.clone();

    println!("########## Seed: raw land, untouched quarry, unmined hall ##########\n");
    print_world(&Materialized::from_commits(&history));

    // --- Negative control 1: quarry stone before Valinor is mountains ---
    println!("\n########## Negative control 1: quarry(stone) before there's a mountain ##########\n");
    let early_quarry_json = fire_json("Valinor/quarry", "quarry", "untouched", "stone", "", "");
    let early_quarry_update = update_from_json(&early_quarry_json).expect("valid DMML");
    let early_quarry_commit = lower_commit(&early_quarry_update.batches[0].commits[0]);
    let world_now = Materialized::from_commits(&history);
    let fired = try_fire("negative control 1", &machines, &world_now, "Valinor/quarry", "quarry", HashMap::new(), &early_quarry_commit);
    assert!(!fired, "quarrying before there's a mountain must be blocked");

    // --- Turn 1: raise the land ---
    println!("\n########## Turn 1: raise ##########\n");
    let raise_json = fire_json("Valinor", "raise", "unformed", "hills", "", "");
    let raise_update = update_from_json(&raise_json).expect("valid DMML");
    let raise_commit = lower_commit(&raise_update.batches[0].commits[0]);
    let world_before_raise = Materialized::from_commits(&history);
    try_fire("turn 1", &machines, &world_before_raise, "Valinor", "raise", HashMap::new(), &raise_commit);
    history.push(raise_commit);

    // --- Turn 2: uplift into mountains ---
    println!("\n########## Turn 2: uplift ##########\n");
    let uplift_json = fire_json("Valinor", "uplift", "hills", "mountains", "", "");
    let uplift_update = update_from_json(&uplift_json).expect("valid DMML");
    let uplift_commit = lower_commit(&uplift_update.batches[0].commits[0]);
    let world_before_uplift = Materialized::from_commits(&history);
    try_fire("turn 2", &machines, &world_before_uplift, "Valinor", "uplift", HashMap::new(), &uplift_commit);
    history.push(uplift_commit);
    print_world(&Materialized::from_commits(&history));

    // --- Negative control 2: carve the hall before there's any stone ---
    println!("\n########## Negative control 2: carve(quarry) before it's stone ##########\n");
    let early_carve_json = fire_json(
        "Valinor/hall", "carve", "unmined", "carved",
        r#", {"subject": "Valinor/hall", "predicate": "material_source", "object": {"kind": "node", "value": "Valinor/quarry"}}"#,
        r#"{"kind": "attribute", "name": "material_source"}"#,
    );
    let early_carve_update = update_from_json(&early_carve_json).expect("valid DMML");
    let early_carve_commit = lower_commit(&early_carve_update.batches[0].commits[0]);
    let source_param = extract_param(&early_carve_commit, "Valinor/hall", "material_source").unwrap();
    let world_now2 = Materialized::from_commits(&history);
    let fired2 = try_fire(
        "negative control 2", &machines, &world_now2, "Valinor/hall", "carve",
        HashMap::from([("material_source".to_string(), source_param)]), &early_carve_commit,
    );
    assert!(!fired2, "carving before there's stone must be blocked");

    // --- Turn 3: quarry stone, now legitimately (Valinor is mountains) ---
    println!("\n########## Turn 3: quarry (Valinor is now mountains) ##########\n");
    let quarry_json = fire_json(
        "Valinor/quarry", "quarry", "untouched", "stone",
        r#", {"subject": "Valinor/quarry", "predicate": "description", "object": {"kind": "str", "value": "a raw grey seam of stone, cut open at the mountain's flank"}}"#,
        r#"{"kind": "attribute", "name": "description"}"#,
    );
    let quarry_update = update_from_json(&quarry_json).expect("valid DMML");
    let quarry_commit = lower_commit(&quarry_update.batches[0].commits[0]);
    let world_before_quarry = Materialized::from_commits(&history);
    let fired3 = try_fire("turn 3", &machines, &world_before_quarry, "Valinor/quarry", "quarry", HashMap::new(), &quarry_commit);
    assert!(fired3, "quarrying should succeed now that Valinor is mountains");
    history.push(quarry_commit);
    print_world(&Materialized::from_commits(&history));

    // --- Turn 4: carve the hall, now legitimately (quarry is stone) ---
    println!("\n########## Turn 4: carve(Valinor/quarry) ##########\n");
    let carve_json = fire_json(
        "Valinor/hall", "carve", "unmined", "carved",
        r#", {"subject": "Valinor/hall", "predicate": "material_source", "object": {"kind": "node", "value": "Valinor/quarry"}}, {"subject": "Valinor/hall", "predicate": "description", "object": {"kind": "str", "value": "a hall opens in the stone, ceiling still rough with chisel-marks"}}"#,
        r#"{"kind": "attribute", "name": "material_source"}, {"kind": "attribute", "name": "description"}"#,
    );
    let carve_update = update_from_json(&carve_json).expect("valid DMML");
    let carve_commit = lower_commit(&carve_update.batches[0].commits[0]);
    let source_param_2 = extract_param(&carve_commit, "Valinor/hall", "material_source").unwrap();
    let world_before_carve = Materialized::from_commits(&history);
    let fired4 = try_fire(
        "turn 4", &machines, &world_before_carve, "Valinor/hall", "carve",
        HashMap::from([("material_source".to_string(), source_param_2)]), &carve_commit,
    );
    assert!(fired4, "carving should succeed now that the quarry is stone");
    history.push(carve_commit);

    // --- Turns 5-7: the rest of the material differentiation chain,
    // shown for its own sake -- stone into sand, sand wet into clay,
    // clay fired into brick. Nothing downstream consumes brick yet in
    // this example; that's the natural next machine (a wall or tower
    // built FROM brick), not built here. ---
    for (label, transition, from_s, to_s, desc) in [
        ("Turn 5", "grind", "stone", "sand", "the exposed stone is ground down to coarse grey sand"),
        ("Turn 6", "wet", "sand", "clay", "wetted, the sand binds into workable clay"),
        ("Turn 7", "fire", "clay", "brick", "fired hard in a pit of coals, the clay becomes brick"),
    ] {
        println!("\n########## {label}: {transition} ##########\n");
        let json = fire_json(
            "Valinor/quarry", transition, from_s, to_s,
            &format!(r#", {{"subject": "Valinor/quarry", "predicate": "description", "object": {{"kind": "str", "value": "{desc}"}}}}"#),
            r#"{"kind": "attribute", "name": "description"}"#,
        );
        let update = update_from_json(&json).expect("valid DMML");
        let commit = lower_commit(&update.batches[0].commits[0]);
        let world_before = Materialized::from_commits(&history);
        let fired = try_fire(label, &machines, &world_before, "Valinor/quarry", transition, HashMap::new(), &commit);
        assert!(fired, "{label} should succeed");
        history.push(commit);
    }

    println!("\n########## Final world state ##########\n");
    let final_world = Materialized::from_commits(&history);
    print_world(&final_world);
    println!("\n{} distinct (subject, predicate) triples across {} commits.", final_world.len(), history.len());
}
