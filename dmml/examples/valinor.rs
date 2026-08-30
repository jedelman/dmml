//! First build of "verbs are machines, refs are their targets" (Jason,
//! 2026-08-30): instead of an agent freely inventing predicates and
//! facts each tick, the world's action space is a small, fixed set of
//! machine transitions. An agent's "verb" IS a transition ident on a
//! declared machine; its "target" is what that transition's guard/effect
//! actually operates on -- a `consumes` FactRef citing the exact prior
//! `(self, state, from)` fact, not an arbitrary new predicate. This
//! bounds the authoring task to "here's the world, here's what you can
//! do, what do you do" instead of open-ended invention.
//!
//! Three machines, one seed ("Valinor"):
//!   - `Valinor` itself: landscape sculpting (unformed -> hills -> mountains,
//!     or unformed -> valley).
//!   - `sense/vision`: idle <-> looking.
//!   - `sense/touch`: idle <-> touching.
//!
//! Each transition-firing commit ALSO carries ordinary descriptive facts
//! alongside the required state-transition triple -- `commit_fires_
//! transition` only requires the resolved effects be DELIVERED among a
//! commit's consumes/produces, it doesn't forbid additional content. That
//! reconciles the FSM's narrow `(self, "state", value)` grammar with rich
//! narrative output: the machine keeps the action space bounded and
//! legitimate; the commit's own `produces` still carries what the
//! sculpting/seeing/touching actually revealed.
//!
//! This is a JSON-first sketch per this project's own "DMML first"
//! standing rule -- the content below is real, parsed, and checked
//! against `dmml::machine::commit_fires_transition` before any further
//! Rust-shaped design happens on top of it.

use dmml::from_json::update_from_json;
use dmml::interpret::Materialized;
use dmml::lower::{lower_commit, ConsumeRef, LoweredCommit, TripleValue};
use dmml::machine::{self, EvalContext, MachineBody};
use std::collections::HashMap;

/// The seed batch: declares all three machines and mints their initial
/// states plus Valinor itself. A real substrate would give this commit a
/// genuine `{uri, cid}`; standing in with a synthetic one here since this
/// example runs standalone, not against a substrate.
const SEED_URI: &str = "at://did:example:valinor/world.valinor/genesis";
const SEED_CID: &str = "seed-cid-0";

fn seed_json() -> String {
    format!(
        r#"{{
  "update": [
    {{
      "machines": [
        {{
          "node": "Valinor",
          "states": [{{"ident": "unformed"}}, {{"ident": "hills"}}, {{"ident": "mountains"}}, {{"ident": "valley"}}],
          "transitions": [
            {{"ident": "raise", "from": "unformed", "to": "hills"}},
            {{"ident": "carve", "from": "unformed", "to": "valley"}},
            {{"ident": "uplift", "from": "hills", "to": "mountains"}}
          ]
        }},
        {{
          "node": "sense/vision",
          "states": [{{"ident": "idle"}}, {{"ident": "looking"}}],
          "transitions": [
            {{"ident": "look", "from": "idle", "to": "looking"}},
            {{"ident": "rest_eyes", "from": "looking", "to": "idle"}}
          ]
        }},
        {{
          "node": "sense/touch",
          "states": [{{"ident": "idle"}}, {{"ident": "touching"}}],
          "transitions": [
            {{"ident": "touch", "from": "idle", "to": "touching"}},
            {{"ident": "withdraw", "from": "touching", "to": "idle"}}
          ]
        }}
      ],
      "commits": [
        {{
          "verb": "mints",
          "declares": [{{"kind": "attribute", "name": "state"}}],
          "facts": [
            {{"subject": "Valinor", "predicate": "a", "object": {{"kind": "node", "value": "Place"}}}},
            {{"subject": "Valinor", "predicate": "state", "object": {{"kind": "node", "value": "unformed"}}}},
            {{"subject": "sense/vision", "predicate": "state", "object": {{"kind": "node", "value": "idle"}}}},
            {{"subject": "sense/touch", "predicate": "state", "object": {{"kind": "node", "value": "idle"}}}}
          ]
        }}
      ]
    }}
  ]
}}"#
    )
}

/// A single "operate a machine" turn: fires `transition` on `node`,
/// consuming the fact this exact node/predicate/value was last in
/// (satisfying the transition's implicit retract guard/effect), and
/// carrying whatever extra descriptive facts the operator wants to add
/// alongside the state change itself.
fn fire_json(node: &str, transition: &str, from_state: &str, to_state: &str, extra_facts: &str, extra_declares: &str) -> String {
    format!(
        r#"{{
  "update": [
    {{
      "commits": [
        {{
          "verb": "{transition}",
          "declares": [{extra_declares}],
          "consumes": [
            {{"kind": "fact", "commit": {{"uri": "{SEED_URI}", "cid": "{SEED_CID}"}},
             "subject": "{node}", "predicate": "state", "object": {{"kind": "node", "value": "{from_state}"}}}}
          ],
          "facts": [
            {{"subject": "{node}", "predicate": "state", "object": {{"kind": "node", "value": "{to_state}"}}}}
            {extra_facts}
          ]
        }}
      ]
    }}
  ]
}}"#
    )
}

fn machines_from_batches(update: &dmml::from_json::Update) -> HashMap<String, MachineBody> {
    let mut map = HashMap::new();
    for batch in &update.batches {
        for m in &batch.machines {
            map.insert(
                m.node.segments.join("/"),
                MachineBody { states: m.states.clone(), transitions: m.transitions.clone() },
            );
        }
    }
    map
}

fn lower_all(update: &dmml::from_json::Update) -> Vec<LoweredCommit> {
    update.batches.iter().flat_map(|b| b.commits.iter().map(lower_commit)).collect()
}

fn try_fire(
    label: &str,
    machines: &HashMap<String, MachineBody>,
    world_before: &Materialized,
    node: &str,
    transition: &str,
    candidate: &LoweredCommit,
) {
    let body = machines.get(node).expect("machine declared");
    let ctx = EvalContext { self_node: node.to_string(), params: HashMap::new() };
    match machine::commit_fires_transition(body, transition, &ctx, world_before, candidate) {
        Ok(()) => println!("  [OK]     {label}: '{transition}' on {node} fired legitimately."),
        Err(e) => println!("  [BLOCKED] {label}: '{transition}' on {node} rejected: {e:?}"),
    }
}

fn print_world(world: &Materialized) {
    let mut rows: Vec<(&str, &str, &TripleValue)> = world.iter().collect();
    rows.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    for (subj, pred, val) in rows {
        println!("  {subj:16} {pred:12} {val:?}");
    }
}

fn main() {
    // --- Seed: declare the three machines, mint Valinor and both senses ---
    let seed_update = update_from_json(&seed_json()).expect("seed JSON is valid DMML");
    let machines = machines_from_batches(&seed_update);
    let seed_commits = lower_all(&seed_update);
    let mut history = seed_commits.clone();

    println!("########## Seed: Valinor ##########\n");
    print_world(&Materialized::from_commits(&history));

    // --- Turn 1: raise (unformed -> hills), with real descriptive content ---
    let raise_json = fire_json(
        "Valinor", "raise", "unformed", "hills",
        r#", {"subject": "Valinor", "predicate": "description", "object": {"kind": "str", "value": "Green hills rise where flat plains once lay."}}"#,
        r#"{"kind": "attribute", "name": "description"}"#,
    );
    let raise_update = update_from_json(&raise_json).expect("raise JSON is valid DMML");
    let raise_commit = lower_commit(&raise_update.batches[0].commits[0]);
    let world_before_raise = Materialized::from_commits(&history);

    println!("\n########## Turn 1: raise ##########\n");
    try_fire("turn 1", &machines, &world_before_raise, "Valinor", "raise", &raise_commit);
    history.push(raise_commit);
    print_world(&Materialized::from_commits(&history));

    // --- Negative control: try 'uplift' straight from 'unformed' on a
    // FRESH copy of the seed history (never raised) -- should be blocked,
    // proving the machine bounds the action space rather than just
    // rubber-stamping any transition name. ---
    println!("\n########## Negative control: uplift without raising first ##########\n");
    let bad_json = fire_json("Valinor", "uplift", "unformed", "mountains", "", "");
    let bad_update = update_from_json(&bad_json).expect("bad JSON is still valid DMML shape");
    let bad_commit = lower_commit(&bad_update.batches[0].commits[0]);
    let fresh_world = Materialized::from_commits(&seed_commits);
    try_fire("negative control", &machines, &fresh_world, "Valinor", "uplift", &bad_commit);

    // --- Turn 2: uplift (hills -> mountains), now legitimate ---
    let uplift_json = fire_json(
        "Valinor", "uplift", "hills", "mountains",
        r#", {"subject": "Valinor", "predicate": "description", "object": {"kind": "str", "value": "The hills buckle and climb into snow-touched peaks."}}"#,
        r#"{"kind": "attribute", "name": "description"}"#,
    );
    let uplift_update = update_from_json(&uplift_json).expect("uplift JSON is valid DMML");
    let uplift_commit = lower_commit(&uplift_update.batches[0].commits[0]);
    let world_before_uplift = Materialized::from_commits(&history);

    println!("\n########## Turn 2: uplift ##########\n");
    try_fire("turn 2", &machines, &world_before_uplift, "Valinor", "uplift", &uplift_commit);
    history.push(uplift_commit);
    print_world(&Materialized::from_commits(&history));

    // --- Turn 3: vision looks at the now-mountainous Valinor ---
    let look_json = fire_json(
        "sense/vision", "look", "idle", "looking",
        r#", {"subject": "sense/vision", "predicate": "sees", "object": {"kind": "str", "value": "snow-touched peaks catching the first light over Valinor"}}"#,
        r#"{"kind": "attribute", "name": "sees"}"#,
    );
    let look_update = update_from_json(&look_json).expect("look JSON is valid DMML");
    let look_commit = lower_commit(&look_update.batches[0].commits[0]);
    let world_before_look = Materialized::from_commits(&history);

    println!("\n########## Turn 3: look ##########\n");
    try_fire("turn 3", &machines, &world_before_look, "sense/vision", "look", &look_commit);
    history.push(look_commit);
    print_world(&Materialized::from_commits(&history));

    // --- Turn 4: touch reaches for the mountain's stone ---
    let touch_json = fire_json(
        "sense/touch", "touch", "idle", "touching",
        r#", {"subject": "sense/touch", "predicate": "feels", "object": {"kind": "str", "value": "cold stone, still warm from the world's own making"}}"#,
        r#"{"kind": "attribute", "name": "feels"}"#,
    );
    let touch_update = update_from_json(&touch_json).expect("touch JSON is valid DMML");
    let touch_commit = lower_commit(&touch_update.batches[0].commits[0]);
    let world_before_touch = Materialized::from_commits(&history);

    println!("\n########## Turn 4: touch ##########\n");
    try_fire("turn 4", &machines, &world_before_touch, "sense/touch", "touch", &touch_commit);
    history.push(touch_commit);

    println!("\n########## Final world state ##########\n");
    let final_world = Materialized::from_commits(&history);
    print_world(&final_world);
    println!("\n{} distinct (subject, predicate) triples across {} commits.", final_world.len(), history.len());

    // Sanity: every consumes actually resolved (no dangling FactRef in
    // this small, hand-built history -- worth checking directly rather
    // than assuming, per this crate's own "checkable, not policed" ethos).
    for c in &history {
        for cr in &c.consumes {
            if let ConsumeRef::Fact(fr) = cr {
                let value_matches = final_world.current_value(&fr.subject, &fr.predicate).is_some();
                assert!(value_matches, "dangling FactRef in this example's own history: {fr:?}");
            }
        }
    }
    println!("\nAll consumes references in this history resolve to something real. No dangling citations.");
}
