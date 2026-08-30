//! First of the "minimum viable world" starter machines mined from real
//! agent commentary (see this session's conversation, and `GROUNDING-
//! 2026-08-30-amber-cracks.md`'s neurosis-test investigation): a `door`
//! machine, locked/unlocked/open, gated by a specific key's real state --
//! not by an agent freely asserting "the door is unlocked now." Two
//! independent agents reached for a locked-door-and-key mechanic
//! unprompted in earlier runs (`delta`'s "old iron key... glinting with
//! amber reflection" and "rusty iron key in torch bracket"; `gamma`'s
//! "key riddle"); this is that mechanic built as a real, checked machine
//! rather than something every agent has to reinvent from scratch.
//!
//! Extends `valinor.rs`'s world rather than floating context-free: the
//! gate sits at `Valinor/gate`, the key at `key/silver`.
//!
//! Two machines:
//!   - `key/silver`: hidden -> found (a `find` transition, no guard --
//!     anyone can stumble on it).
//!   - `Valinor/gate`: locked -> unlocked -> open. `unlock` takes a
//!     `$key` param and is guarded by `EXISTS($key --state--> found)` --
//!     a FIXED target value (`Node("found")`), not a free existential
//!     (`Var`) -- the gate doesn't care merely that the cited key HAS
//!     some state, it requires that state be specifically `found`. This
//!     is the real content of "gated by a key," expressed as a guard a
//!     resolver checks, not as narration a reader has to trust.

use dmml::from_json::update_from_json;
use dmml::interpret::Materialized;
use dmml::lower::{lower_commit, LoweredCommit, TripleValue};
use dmml::machine::{self, EvalContext, MachineBody};
use std::collections::HashMap;

const SEED_URI: &str = "at://did:example:valinor/world.valinor/gate-genesis";
const SEED_CID: &str = "gate-seed-cid-0";

fn seed_json() -> String {
    format!(
        r#"{{
  "update": [
    {{
      "machines": [
        {{
          "node": "key/silver",
          "states": [{{"ident": "hidden"}}, {{"ident": "found"}}],
          "transitions": [
            {{"ident": "find", "from": "hidden", "to": "found"}}
          ]
        }},
        {{
          "node": "Valinor/gate",
          "states": [{{"ident": "locked"}}, {{"ident": "unlocked"}}, {{"ident": "open"}}],
          "transitions": [
            {{"ident": "unlock", "params": ["key"], "from": "locked", "to": "unlocked",
              "guards": [{{"exists": {{"anchor": {{"kind": "param", "value": "key"}},
                "hops": [{{"predicate": "state", "term": {{"kind": "node", "value": "found"}}}}]}}}}]}},
            {{"ident": "open", "from": "unlocked", "to": "open"}}
          ]
        }}
      ],
      "commits": [
        {{
          "verb": "mints",
          "declares": [{{"kind": "attribute", "name": "state"}}],
          "facts": [
            {{"subject": "key/silver", "predicate": "state", "object": {{"kind": "node", "value": "hidden"}}}},
            {{"subject": "Valinor/gate", "predicate": "state", "object": {{"kind": "node", "value": "locked"}}}},
            {{"subject": "Valinor/gate", "predicate": "a", "object": {{"kind": "node", "value": "Door"}}}}
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

fn try_fire(label: &str, machines: &HashMap<String, MachineBody>, world_before: &Materialized, node: &str, transition: &str, params: HashMap<String, String>, candidate: &LoweredCommit) -> bool {
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

    println!("########## Seed: the gate and its key ##########\n");
    print_world(&Materialized::from_commits(&history));

    // --- Negative control FIRST: try to unlock the gate while the key
    // is still hidden. Should be blocked -- the guard checks the key's
    // ACTUAL state, not just that a key was named. ---
    println!("\n########## Negative control: unlock(key/silver) while it's still hidden ##########\n");
    let early_unlock_json = fire_json(
        "Valinor/gate", "unlock", "locked", "unlocked",
        r#", {"subject": "Valinor/gate", "predicate": "key", "object": {"kind": "node", "value": "key/silver"}}"#,
        r#"{"kind": "attribute", "name": "key"}"#,
    );
    let early_unlock_update = update_from_json(&early_unlock_json).expect("valid DMML");
    let early_unlock_commit = lower_commit(&early_unlock_update.batches[0].commits[0]);
    let key_param = extract_param(&early_unlock_commit, "Valinor/gate", "key").unwrap();
    let world_now = Materialized::from_commits(&history);
    try_fire(
        "negative control", &machines, &world_now, "Valinor/gate", "unlock",
        HashMap::from([("key".to_string(), key_param)]), &early_unlock_commit,
    );

    // --- Turn 1: find the key ---
    println!("\n########## Turn 1: find the silver key ##########\n");
    let find_json = fire_json(
        "key/silver", "find", "hidden", "found",
        r#", {"subject": "key/silver", "predicate": "description", "object": {"kind": "str", "value": "a silver key, half-buried in root and moss at the foot of the gate"}}"#,
        r#"{"kind": "attribute", "name": "description"}"#,
    );
    let find_update = update_from_json(&find_json).expect("valid DMML");
    let find_commit = lower_commit(&find_update.batches[0].commits[0]);
    let world_before_find = Materialized::from_commits(&history);
    try_fire("turn 1", &machines, &world_before_find, "key/silver", "find", HashMap::new(), &find_commit);
    history.push(find_commit);
    print_world(&Materialized::from_commits(&history));

    // --- Turn 2: unlock the gate, now legitimately ---
    println!("\n########## Turn 2: unlock(key/silver), now that it's found ##########\n");
    let unlock_json = fire_json(
        "Valinor/gate", "unlock", "locked", "unlocked",
        r#", {"subject": "Valinor/gate", "predicate": "key", "object": {"kind": "node", "value": "key/silver"}}, {"subject": "Valinor/gate", "predicate": "description", "object": {"kind": "str", "value": "the gate groans, its lock giving way to the silver key"}}"#,
        r#"{"kind": "attribute", "name": "key"}, {"kind": "attribute", "name": "description"}"#,
    );
    let unlock_update = update_from_json(&unlock_json).expect("valid DMML");
    let unlock_commit = lower_commit(&unlock_update.batches[0].commits[0]);
    let key_param_2 = extract_param(&unlock_commit, "Valinor/gate", "key").unwrap();
    let world_before_unlock = Materialized::from_commits(&history);
    let fired = try_fire(
        "turn 2", &machines, &world_before_unlock, "Valinor/gate", "unlock",
        HashMap::from([("key".to_string(), key_param_2)]), &unlock_commit,
    );
    assert!(fired, "unlock should succeed now that the key is found");
    history.push(unlock_commit);
    print_world(&Materialized::from_commits(&history));

    // --- Turn 3: open the now-unlocked gate ---
    println!("\n########## Turn 3: open the gate ##########\n");
    let open_json = fire_json(
        "Valinor/gate", "open", "unlocked", "open",
        r#", {"subject": "Valinor/gate", "predicate": "description", "object": {"kind": "str", "value": "the gate swings inward on old, patient hinges"}}"#,
        r#"{"kind": "attribute", "name": "description"}"#,
    );
    let open_update = update_from_json(&open_json).expect("valid DMML");
    let open_commit = lower_commit(&open_update.batches[0].commits[0]);
    let world_before_open = Materialized::from_commits(&history);
    try_fire("turn 3", &machines, &world_before_open, "Valinor/gate", "open", HashMap::new(), &open_commit);
    history.push(open_commit);

    println!("\n########## Final world state ##########\n");
    let final_world = Materialized::from_commits(&history);
    print_world(&final_world);
    println!("\n{} distinct (subject, predicate) triples across {} commits.", final_world.len(), history.len());
}
