//! The first Valar-minted machine, verified. "I think this an actual
//! model quality question!" (Jason, 2026-08-30) -- and it was: three
//! attempts at the Vala design task (deepseek-v4-flash-0731 x3, then
//! z-ai/glm-5.3) produced either no guards at all or an ungrounded,
//! invented material (`glass_source`/`crystal_source`, never mentioned
//! anywhere in the world). `openai/gpt-5.2-pro`'s attempt was a
//! different quality of result entirely: `Valinor/kiln` (built from
//! brick + mortar, a real two-input join) and `Valinor/pottery`
//! (raw -> shaped, via clay + water; shaped -> fired, via the kiln) --
//! genuinely grounded, genuinely extending the existing brick/mortar
//! economy in a new direction.
//!
//! It still failed validation on two concrete bugs, both fixed by hand,
//! neither a design failure:
//!   1. Wrapped `"update"` as an object (`{"commits":[],"machines":
//!      [...]}`) instead of the required array-of-batches shape
//!      (`[{"commits":[],"machines":[...]}]`) -- a real schema-adherence
//!      miss even with native structured-output support.
//!   2. Guessed the wrong predicate for state checks (`"a"`/rdf:type
//!      instead of `"state"`) -- and its own reasoning trace shows it
//!      explicitly agonizing over exactly this ambiguity ("I wonder if
//!      DMML records machine state as rdf:type or 'state'?"). This one
//!      is partly the prompt's fault: `valar_mint.py`'s world
//!      description never stated the guard-predicate convention
//!      explicitly, only implied it through examples.
//!
//! Once both were corrected (`VALAR-MINTED-2026-08-30-openai-gpt-52-pro-
//! FIXED.json`), it validated clean on the first try -- proving the
//! underlying design was sound, not merely salvageable. This file wires
//! that corrected design into `house.rs`'s world and fires it for real:
//! quarry/mortar produce the kiln's inputs, the kiln gets built, then
//! clay + spring water shape raw pottery, and the built kiln fires it.

use dmml::from_json::update_from_json;
use dmml::interpret::Materialized;
use dmml::lower::{lower_commit, LoweredCommit, TripleValue};
use dmml::machine::{self, EvalContext, MachineBody};
use std::collections::HashMap;

const SEED_URI: &str = "at://did:example:valinor/world.valinor/kiln-genesis";
const SEED_CID: &str = "kiln-seed-cid-0";

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
          "node": "Valinor/quarry2",
          "states": [{{"ident": "untouched"}}, {{"ident": "stone"}}, {{"ident": "sand"}}, {{"ident": "clay"}}, {{"ident": "brick"}}],
          "transitions": [
            {{"ident": "grind2", "from": "untouched", "to": "sand"}},
            {{"ident": "wet2", "from": "sand", "to": "clay"}},
            {{"ident": "fire2", "from": "clay", "to": "brick"}}
          ]
        }},
        {{
          "node": "Valinor/kiln",
          "states": [{{"ident": "unbuilt_kiln"}}, {{"ident": "built_kiln"}}],
          "transitions": [
            {{"ident": "build_kiln", "params": ["brick_source", "mortar_source"], "from": "unbuilt_kiln", "to": "built_kiln",
              "guards": [
                {{"exists": {{"anchor": {{"kind": "param", "value": "brick_source"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "brick"}}}}]}}}},
                {{"exists": {{"anchor": {{"kind": "param", "value": "mortar_source"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "mixed"}}}}]}}}}
              ]}}
          ]
        }},
        {{
          "node": "Valinor/pottery",
          "states": [{{"ident": "raw_pottery"}}, {{"ident": "shaped_pottery"}}, {{"ident": "fired_pottery"}}],
          "transitions": [
            {{"ident": "shape_pottery", "params": ["clay_source", "water_source"], "from": "raw_pottery", "to": "shaped_pottery",
              "guards": [
                {{"exists": {{"anchor": {{"kind": "param", "value": "clay_source"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "clay"}}}}]}}}},
                {{"exists": {{"anchor": {{"kind": "param", "value": "water_source"}},
                  "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "flowing"}}}}]}}}}
              ]}},
            {{"ident": "fire_pottery", "params": ["kiln_source"], "from": "shaped_pottery", "to": "fired_pottery",
              "guards": [{{"exists": {{"anchor": {{"kind": "param", "value": "kiln_source"}},
                "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "built_kiln"}}}}]}}}}]}}
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
            {{"subject": "Valinor/quarry2", "predicate": "state", "object": {{"kind": "node", "value": "untouched"}}}},
            {{"subject": "Valinor/kiln", "predicate": "state", "object": {{"kind": "node", "value": "unbuilt_kiln"}}}},
            {{"subject": "Valinor/pottery", "predicate": "state", "object": {{"kind": "node", "value": "raw_pottery"}}}}
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

fn fire_params(
    history: &mut Vec<LoweredCommit>,
    machines: &HashMap<String, MachineBody>,
    label: &str,
    node: &str,
    transition: &str,
    from_s: &str,
    to_s: &str,
    params: &[(&str, &str)],
    expect_fire: bool,
) {
    let extra: String = params
        .iter()
        .map(|(name, value)| format!(r#", {{"subject": "{node}", "predicate": "{name}", "object": {{"kind": "node", "value": "{value}"}}}}"#))
        .collect();
    let declares: String = params
        .iter()
        .map(|(name, _)| format!(r#"{{"kind": "attribute", "name": "{name}"}}"#))
        .collect::<Vec<_>>()
        .join(", ");
    let json = fire_json(node, transition, from_s, to_s, &extra, &declares);
    let update = update_from_json(&json).expect("valid DMML");
    let commit = lower_commit(&update.batches[0].commits[0]);
    let mut ctx_params = HashMap::new();
    for (name, _) in params {
        ctx_params.insert(name.to_string(), extract_param(&commit, node, name).unwrap());
    }
    let world_before = Materialized::from_commits(history);
    let fired = try_fire(label, machines, &world_before, node, transition, ctx_params, &commit);
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

    println!("\n########## Build the kiln's own inputs (brick + mortar) ##########\n");
    fire_simple(&mut history, &machines, "raise", "Valinor", "raise", "unformed", "hills");
    fire_simple(&mut history, &machines, "uplift", "Valinor", "uplift", "hills", "mountains");
    fire_simple(&mut history, &machines, "quarry", "Valinor/quarry", "quarry", "untouched", "stone");
    fire_simple(&mut history, &machines, "grind", "Valinor/quarry", "grind", "stone", "sand");
    fire_simple(&mut history, &machines, "wet", "Valinor/quarry", "wet", "sand", "clay");
    fire_simple(&mut history, &machines, "fire", "Valinor/quarry", "fire", "clay", "brick");
    fire_simple(&mut history, &machines, "wash", "Valinor/streambed", "wash", "bare", "sand");
    fire_simple(&mut history, &machines, "well_up", "Valinor/spring", "well_up", "dry", "flowing");
    fire_params(&mut history, &machines, "mix", "Valinor/mortar", "mix", "unmixed", "mixed",
        &[("sand_source", "Valinor/streambed"), ("water_source", "Valinor/spring")], true);

    // Second quarry, kept at "clay" specifically for pottery -- Valinor/
    // quarry itself is now sitting at "brick" and can't simultaneously
    // supply clay too, the same single-mutable-resource limitation
    // wall.rs's own doc comment already named honestly.
    fire_simple(&mut history, &machines, "grind2", "Valinor/quarry2", "grind2", "untouched", "sand");
    fire_simple(&mut history, &machines, "wet2", "Valinor/quarry2", "wet2", "sand", "clay");

    println!("\n########## Negative control: build the kiln before mortar is mixed... already mixed here, so instead: fire pottery before the kiln exists ##########\n");
    let early_fire_json = fire_json(
        "Valinor/pottery", "fire_pottery", "shaped_pottery", "fired_pottery",
        r#", {"subject": "Valinor/pottery", "predicate": "kiln_source", "object": {"kind": "node", "value": "Valinor/kiln"}}"#,
        r#"{"kind": "attribute", "name": "kiln_source"}"#,
    );
    let early_fire_update = update_from_json(&early_fire_json).expect("valid DMML");
    let early_fire_commit = lower_commit(&early_fire_update.batches[0].commits[0]);
    let kiln_src = extract_param(&early_fire_commit, "Valinor/pottery", "kiln_source").unwrap();
    let world_now = Materialized::from_commits(&history);
    let fired_early = try_fire(
        "negative control", &machines, &world_now, "Valinor/pottery", "fire_pottery",
        HashMap::from([("kiln_source".to_string(), kiln_src)]), &early_fire_commit,
    );
    assert!(!fired_early, "firing pottery before the kiln is built must be blocked");

    println!("\n########## Build the kiln (brick + mortar, a real two-input join) ##########\n");
    fire_params(&mut history, &machines, "build_kiln", "Valinor/kiln", "build_kiln", "unbuilt_kiln", "built_kiln",
        &[("brick_source", "Valinor/quarry"), ("mortar_source", "Valinor/mortar")], true);
    print_world(&Materialized::from_commits(&history));

    println!("\n########## Shape pottery (clay + water) ##########\n");
    fire_params(&mut history, &machines, "shape_pottery", "Valinor/pottery", "shape_pottery", "raw_pottery", "shaped_pottery",
        &[("clay_source", "Valinor/quarry2"), ("water_source", "Valinor/spring")], true);

    println!("\n########## Fire the shaped pottery in the now-built kiln ##########\n");
    fire_params(&mut history, &machines, "fire_pottery", "Valinor/pottery", "fire_pottery", "shaped_pottery", "fired_pottery",
        &[("kiln_source", "Valinor/kiln")], true);

    println!("\n########## Final world state ##########\n");
    let final_world = Materialized::from_commits(&history);
    print_world(&final_world);
    println!("\n{} distinct (subject, predicate) triples across {} commits.", final_world.len(), history.len());
    println!("\nA Vala's own design, corrected of two surface bugs, fires cleanly end to end.");
}
