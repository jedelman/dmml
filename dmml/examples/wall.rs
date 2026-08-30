//! Two-input consumption, then the wall that finally consumes `brick`
//! (Jason, 2026-08-30: "both -- start with two-input consumption").
//! Every guard in `valinor.rs`/`door.rs`/`quarry.rs` checks exactly ONE
//! fact. Real production usually needs more than one input at once --
//! this example builds two genuine two-input joins, chained:
//!
//!   `Valinor/streambed` (bare -> sand) and `Valinor/spring`
//!   (dry -> flowing) are independent, ungated mints -- water and sand,
//!   each on its own.
//!
//!   `Valinor/mortar` (unmixed -> mixed): `mix($sand_source,
//!   $water_source)` carries TWO guards in its list, not one --
//!   `eval_guards` already ANDs across a transition's whole guard list
//!   (nothing new needed there), just never exercised with more than one
//!   clause before. BOTH must hold: the cited sand source must actually
//!   be sand, AND the cited water source must actually be flowing. Water
//!   drying back up after mortar's already mixed doesn't unmix it --
//!   this only checks the instant of firing, same as every other guard
//!   here; that's a real, worth-naming limitation, not glossed over.
//!
//!   `Valinor/wall` (unbuilt -> built): `build($brick_source,
//!   $mortar_source)`, the same two-guard shape one level up the
//!   production chain -- brick alone was never enough to build anything;
//!   it needed something to bind it. This is `quarry.rs`'s stone -> sand
//!   -> clay -> brick chain FINALLY consumed by something, not left
//!   dangling.
//!
//! Full chain end to end: raise/uplift Valinor into mountains, run the
//! quarry chain to brick, wash the streambed to sand, well up the
//! spring -- but mortar is attempted (and blocked) BEFORE the spring
//! flows, to prove both guards are actually checked, not just the first
//! one satisfied. Then the spring flows, mortar mixes, and the wall
//! goes up.

use dmml::from_json::update_from_json;
use dmml::interpret::Materialized;
use dmml::lower::{lower_commit, LoweredCommit, TripleValue};
use dmml::machine::{self, EvalContext, MachineBody};
use std::collections::HashMap;

const SEED_URI: &str = "at://did:example:valinor/world.valinor/wall-genesis";
const SEED_CID: &str = "wall-seed-cid-0";

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
            {{"subject": "Valinor/wall", "predicate": "state", "object": {{"kind": "node", "value": "unbuilt"}}}}
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

/// Fires a plain, ungated (or self-guard-only) state transition and
/// pushes it onto history -- the repeated shape for raise/uplift/quarry/
/// grind/wet/fire/wash/well_up, none of which take params.
fn fire_simple(
    history: &mut Vec<LoweredCommit>,
    machines: &HashMap<String, MachineBody>,
    label: &str,
    node: &str,
    transition: &str,
    from_s: &str,
    to_s: &str,
    desc: Option<&str>,
) {
    let extra = desc
        .map(|d| format!(r#", {{"subject": "{node}", "predicate": "description", "object": {{"kind": "str", "value": "{d}"}}}}"#))
        .unwrap_or_default();
    let declares = if desc.is_some() { r#"{"kind": "attribute", "name": "description"}"# } else { "" };
    let json = fire_json(node, transition, from_s, to_s, &extra, declares);
    let update = update_from_json(&json).expect("valid DMML");
    let commit = lower_commit(&update.batches[0].commits[0]);
    let world_before = Materialized::from_commits(history);
    let fired = try_fire(label, machines, &world_before, node, transition, HashMap::new(), &commit);
    assert!(fired, "{label} ({transition} on {node}) should succeed");
    history.push(commit);
}

fn main() {
    let seed_update = update_from_json(&seed_json()).expect("seed JSON is valid DMML");
    let machines = machines_from_batches(&seed_update);
    let mut history: Vec<LoweredCommit> = seed_update.batches.iter().flat_map(|b| b.commits.iter().map(lower_commit)).collect();

    println!("########## Seed ##########\n");
    print_world(&Materialized::from_commits(&history));

    println!("\n########## Building up to brick and sand ##########\n");
    fire_simple(&mut history, &machines, "raise", "Valinor", "raise", "unformed", "hills", None);
    fire_simple(&mut history, &machines, "uplift", "Valinor", "uplift", "hills", "mountains", None);
    fire_simple(&mut history, &machines, "quarry", "Valinor/quarry", "quarry", "untouched", "stone", None);
    fire_simple(&mut history, &machines, "grind", "Valinor/quarry", "grind", "stone", "sand", None);
    fire_simple(&mut history, &machines, "wet", "Valinor/quarry", "wet", "sand", "clay", None);
    fire_simple(&mut history, &machines, "fire", "Valinor/quarry", "fire", "clay", "brick", None);
    fire_simple(&mut history, &machines, "wash", "Valinor/streambed", "wash", "bare", "sand", None);
    print_world(&Materialized::from_commits(&history));

    // --- Negative control: mix mortar while the spring is still dry --
    // sand is ready but water isn't. Both guards must hold; only one
    // does. ---
    println!("\n########## Negative control: mix(streambed, spring) before the spring flows ##########\n");
    let early_mix_json = fire_json(
        "Valinor/mortar", "mix", "unmixed", "mixed",
        r#", {"subject": "Valinor/mortar", "predicate": "sand_source", "object": {"kind": "node", "value": "Valinor/streambed"}}, {"subject": "Valinor/mortar", "predicate": "water_source", "object": {"kind": "node", "value": "Valinor/spring"}}"#,
        r#"{"kind": "attribute", "name": "sand_source"}, {"kind": "attribute", "name": "water_source"}"#,
    );
    let early_mix_update = update_from_json(&early_mix_json).expect("valid DMML");
    let early_mix_commit = lower_commit(&early_mix_update.batches[0].commits[0]);
    let sand_src = extract_param(&early_mix_commit, "Valinor/mortar", "sand_source").unwrap();
    let water_src = extract_param(&early_mix_commit, "Valinor/mortar", "water_source").unwrap();
    let world_now = Materialized::from_commits(&history);
    let fired = try_fire(
        "negative control", &machines, &world_now, "Valinor/mortar", "mix",
        HashMap::from([("sand_source".to_string(), sand_src.clone()), ("water_source".to_string(), water_src.clone())]),
        &early_mix_commit,
    );
    assert!(!fired, "mixing mortar before the spring flows must be blocked (only one of two guards holds)");

    fire_simple(&mut history, &machines, "well_up", "Valinor/spring", "well_up", "dry", "flowing", None);

    // --- Turn: mix mortar, now legitimately -- both sand and water ready ---
    println!("\n########## Turn: mix(streambed, spring), now both are ready ##########\n");
    let mix_json = fire_json(
        "Valinor/mortar", "mix", "unmixed", "mixed",
        r#", {"subject": "Valinor/mortar", "predicate": "sand_source", "object": {"kind": "node", "value": "Valinor/streambed"}}, {"subject": "Valinor/mortar", "predicate": "water_source", "object": {"kind": "node", "value": "Valinor/spring"}}, {"subject": "Valinor/mortar", "predicate": "description", "object": {"kind": "str", "value": "grey sand and spring water bind into wet, workable mortar"}}"#,
        r#"{"kind": "attribute", "name": "sand_source"}, {"kind": "attribute", "name": "water_source"}, {"kind": "attribute", "name": "description"}"#,
    );
    let mix_update = update_from_json(&mix_json).expect("valid DMML");
    let mix_commit = lower_commit(&mix_update.batches[0].commits[0]);
    let sand_src2 = extract_param(&mix_commit, "Valinor/mortar", "sand_source").unwrap();
    let water_src2 = extract_param(&mix_commit, "Valinor/mortar", "water_source").unwrap();
    let world_before_mix = Materialized::from_commits(&history);
    let fired_mix = try_fire(
        "mix", &machines, &world_before_mix, "Valinor/mortar", "mix",
        HashMap::from([("sand_source".to_string(), sand_src2), ("water_source".to_string(), water_src2)]),
        &mix_commit,
    );
    assert!(fired_mix, "mixing mortar should succeed now that both sand and water are ready");
    history.push(mix_commit);
    print_world(&Materialized::from_commits(&history));

    // --- Negative control: build the wall before mortar is mixed
    // (already mixed by now in this history -- demonstrate the OTHER
    // missing-input case instead: brick without mortar, on a fresh
    // pre-mortar snapshot of history). ---
    println!("\n########## Negative control: build(brick, mortar) on brick alone, no mortar yet ##########\n");
    let history_before_mortar: Vec<LoweredCommit> = history[..history.len() - 1].to_vec();
    let early_build_json = fire_json(
        "Valinor/wall", "build", "unbuilt", "built",
        r#", {"subject": "Valinor/wall", "predicate": "brick_source", "object": {"kind": "node", "value": "Valinor/quarry"}}, {"subject": "Valinor/wall", "predicate": "mortar_source", "object": {"kind": "node", "value": "Valinor/mortar"}}"#,
        r#"{"kind": "attribute", "name": "brick_source"}, {"kind": "attribute", "name": "mortar_source"}"#,
    );
    let early_build_update = update_from_json(&early_build_json).expect("valid DMML");
    let early_build_commit = lower_commit(&early_build_update.batches[0].commits[0]);
    let brick_src = extract_param(&early_build_commit, "Valinor/wall", "brick_source").unwrap();
    let mortar_src = extract_param(&early_build_commit, "Valinor/wall", "mortar_source").unwrap();
    let world_pre_mortar = Materialized::from_commits(&history_before_mortar);
    let fired_early_build = try_fire(
        "negative control", &machines, &world_pre_mortar, "Valinor/wall", "build",
        HashMap::from([("brick_source".to_string(), brick_src.clone()), ("mortar_source".to_string(), mortar_src.clone())]),
        &early_build_commit,
    );
    assert!(!fired_early_build, "building before mortar is mixed must be blocked (brick alone isn't enough)");

    // --- Turn: build the wall, now legitimately -- brick AND mortar ready ---
    println!("\n########## Turn: build(quarry, mortar) -- the wall finally rises ##########\n");
    let build_json = fire_json(
        "Valinor/wall", "build", "unbuilt", "built",
        r#", {"subject": "Valinor/wall", "predicate": "brick_source", "object": {"kind": "node", "value": "Valinor/quarry"}}, {"subject": "Valinor/wall", "predicate": "mortar_source", "object": {"kind": "node", "value": "Valinor/mortar"}}, {"subject": "Valinor/wall", "predicate": "description", "object": {"kind": "str", "value": "brick and mortar rise together into a wall, still dark and damp at the seams"}}"#,
        r#"{"kind": "attribute", "name": "brick_source"}, {"kind": "attribute", "name": "mortar_source"}, {"kind": "attribute", "name": "description"}"#,
    );
    let build_update = update_from_json(&build_json).expect("valid DMML");
    let build_commit = lower_commit(&build_update.batches[0].commits[0]);
    let brick_src2 = extract_param(&build_commit, "Valinor/wall", "brick_source").unwrap();
    let mortar_src2 = extract_param(&build_commit, "Valinor/wall", "mortar_source").unwrap();
    let world_before_build = Materialized::from_commits(&history);
    let fired_build = try_fire(
        "build", &machines, &world_before_build, "Valinor/wall", "build",
        HashMap::from([("brick_source".to_string(), brick_src2), ("mortar_source".to_string(), mortar_src2)]),
        &build_commit,
    );
    assert!(fired_build, "building should succeed now that brick and mortar are both ready");
    history.push(build_commit);

    println!("\n########## Final world state ##########\n");
    let final_world = Materialized::from_commits(&history);
    print_world(&final_world);
    println!("\n{} distinct (subject, predicate) triples across {} commits.", final_world.len(), history.len());
}
