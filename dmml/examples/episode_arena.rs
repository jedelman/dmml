//! Round 9: "what I'm interested in is having them interact **in the
//! world**, in order to **enrich its complexity unintentionally**"
//! (Jason, 2026-08-31), after Round 8's swarm produced this project's
//! first real goal-failure (glm-4.7-flash walked into the overgather
//! trap for real). Jason's framing on seeing that: "models can make
//! mistakes in the world! that's okay! it makes the world interesting!
//! it's when they can't form commits that we lose their
//! contributions!" -- i.e. a bad-but-legal choice is real content, not
//! noise to eliminate; a non-conformant response (Round 8's gemini-
//! lite failure mode) is the actual loss, because nothing about it
//! becomes part of the world at all.
//!
//! Every prior round (`episode_driver.rs`, `episode_test*.py`) was one
//! model, one linear turn sequence, a private world per model. Asked
//! how models should share ONE world, Jason picked "parallel race - new
//! commits get broadcast": models act concurrently, not in a fixed
//! rotation, against a single shared, live-changing world -- whoever's
//! proposed commit actually lands first wins that slot; a model whose
//! proposal goes stale because someone else's commit landed first while
//! it was still thinking just loses that attempt (a real, structural
//! consequence of being slower, not a bug to hide).
//!
//! This is a genuinely different shape than a synchronous stdin/stdout
//! turn loop can express, so it's a new binary rather than an
//! extension of `episode_driver.rs` (which stays exactly as it was for
//! the single-agent, turn-by-turn case). `episode_arena.rs` is a tiny
//! TCP server, std-library only (no new dependencies): the shared
//! world (`Arc<Mutex<Vec<LoweredCommit>>>`) lives in the server
//! process; any number of clients connect concurrently, each
//! connection is one request/response (`{"query": true}` to see
//! current state + legal actions, or `{"actor": ..., "node": ...,
//! "transition": ..., "params": ...}` to attempt a real commit) --
//! the mutex lock around "check the guard, then apply" IS the race's
//! actual resolution point: whichever thread acquires it first, while
//! the guard still holds, wins; a proposal built against
//! already-stale state fails `GuardNotSatisfied` the same way it
//! always has, just now because the world moved between when the
//! actor last looked and when its commit actually lands, not because
//! of any single-threaded turn-ordering artifact.
//!
//! No goal, no turn cap, no fixed step count enforced here -- this is
//! deliberately open-ended; whatever bounds the session (wall-clock
//! time, a target number of committed actions) belongs to the client
//! orchestrator, not the world itself.
//!
//! Usage: `cargo run -p dmml --example episode_arena -- [port]`
//! (defaults to 7878).

//! Extended 2026-08-31, Round 14: Round 13 triangulated the operate-
//! swarm's conformance collapse down to one real driver -- state
//! changing for reasons the querying agent didn't cause and couldn't
//! predict, not mere concurrency or mere evolution over time -- and
//! named "give it slightly more, but structured, not narrated" (drift
//! attribution instead of a bare fresh snapshot) as the mitigation
//! worth testing rather than assuming. Jason's own reminder made the
//! primitive obvious: `dmml::interpret::diverges` already exists,
//! already proven (`dmml/examples/drift_machine.rs`, CLAUDE.md's
//! "DMML first" section) -- exactly the "what changed between two
//! materialized snapshots" comparison this needed, not a new ad hoc
//! diff.
//!
//! The server now tracks each actor's own last-seen `Materialized`
//! snapshot (`Arc<Mutex<HashMap<String, Materialized>>>`, keyed by the
//! `actor` string every request already carries). On `Query`, before
//! updating that actor's record to the current world, it calls
//! `diverges(&previous, &current)` and returns the result as
//! `changed_since_you_last_looked` -- a real, computed answer to
//! "what happened since I last looked that I didn't cause myself,"
//! not a narrated warning that the world *might* have changed (which
//! Round 11 already showed doesn't move the number on its own). An
//! actor's first-ever query reports no drift (nothing to compare
//! against yet), not a flood of "everything just appeared."

use dmml::from_json::update_from_json;
use dmml::interpret::{diverges, Materialized};
use dmml::lower::{ConsumeRef, FactRef, LoweredCommit, StrongRef, Triple, TripleValue};
use dmml::machine::{self, EvalContext, MachineBody};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

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
          "transitions": [
            {"ident": "wash", "from": "bare", "to": "sand",
              "guards": [{"negated": true, "exists": {"anchor": {"kind": "node", "value": "Valinor/forest"},
                "hops": [{"predicate": "state", "term": {"kind": "node", "value": "depleted"}}]}}]}]},
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
#[serde(untagged)]
enum Request {
    Query { query: bool, actor: String },
    Act { actor: String, node: String, transition: String, #[serde(default)] params: Option<HashMap<String, String>> },
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

fn build_candidate(decl: &dmml::machine::TransitionDecl, node: &str, params: &HashMap<String, String>) -> LoweredCommit {
    let dummy_ref = StrongRef { uri: "at://did:example:arena/world.arena/commit".to_string(), cid: "arena-cid".to_string() };
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

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

fn handle_connection(
    stream: TcpStream,
    machines: Arc<HashMap<String, MachineBody>>,
    history: Arc<Mutex<Vec<LoweredCommit>>>,
    commit_log: Arc<Mutex<Vec<serde_json::Value>>>,
    last_seen: Arc<Mutex<HashMap<String, Materialized>>>,
) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let mut stream = stream;

    let req: Request = match serde_json::from_str(&line) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(stream, "{}", serde_json::json!({"error": format!("bad request: {e}")}));
            return;
        }
    };

    match req {
        Request::Query { actor, .. } => {
            let hist = history.lock().unwrap();
            let world = Materialized::from_commits(&hist);
            let actions = legal_actions(&machines, &world);
            let hist_len = hist.len();
            drop(hist);

            // Structured drift attribution (Round 14): what changed
            // since THIS actor last looked, computed via the real
            // dmml::interpret::diverges primitive -- not a narrated
            // warning that the world might have moved, an actual
            // diff. First-ever query for an actor has nothing to
            // compare against, so it reports no drift rather than
            // flooding with "everything just appeared."
            let mut seen = last_seen.lock().unwrap();
            let changes: Vec<serde_json::Value> = match seen.get(&actor) {
                Some(previous) => diverges(previous, &world)
                    .into_iter()
                    .map(|d| serde_json::json!({
                        "subject": d.subject,
                        "predicate": d.predicate,
                        "before": d.before,
                        "after": d.after,
                    }))
                    .collect(),
                None => Vec::new(),
            };
            seen.insert(actor, world.clone());
            drop(seen);

            let resp = serde_json::json!({
                "state": state_snapshot(&world),
                "legal_actions": actions,
                "history_len": hist_len,
                "changed_since_you_last_looked": changes,
            });
            let _ = writeln!(stream, "{}", resp);
        }
        Request::Act { actor, node, transition, params } => {
            let params = params.unwrap_or_default();
            // The lock IS the race's resolution point: whichever thread
            // gets here first, while the guard still holds against
            // whatever's actually in `history` at that instant, wins.
            let mut hist = history.lock().unwrap();
            let world_before = Materialized::from_commits(&hist);

            let result: Result<HashMap<String, String>, String> = (|| {
                let body = machines.get(&node).ok_or_else(|| format!("no machine named '{node}'"))?;
                let decl = body.transitions.iter().find(|t| t.ident == transition)
                    .ok_or_else(|| format!("'{node}' has no transition '{transition}'"))?;
                let candidate = build_candidate(decl, &node, &params);
                let ctx = EvalContext { self_node: node.clone(), params: params.clone() };
                machine::commit_fires_transition(body, &transition, &ctx, &world_before, &candidate)
                    .map_err(|e| format!("{e:?}"))?;
                hist.push(candidate);
                let world_after = Materialized::from_commits(&hist);
                Ok(state_snapshot(&world_after))
            })();

            let (fire_result, state) = match &result {
                Ok(state) => ("PASS".to_string(), state.clone()),
                Err(e) => (format!("FAIL: {e}"), state_snapshot(&world_before)),
            };
            let commit_index = hist.len();
            drop(hist);

            let entry = serde_json::json!({
                "t_ms": now_ms(),
                "actor": actor,
                "node": node,
                "transition": transition,
                "params": params,
                "fire_result": fire_result,
                "commit_index": commit_index,
            });
            commit_log.lock().unwrap().push(entry.clone());

            let resp = serde_json::json!({"fire_result": fire_result, "state": state, "commit_index": commit_index});
            let _ = writeln!(stream, "{}", resp);
            eprintln!("{}", entry);
        }
    }
}

fn main() {
    let port: u16 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(7878);

    let update = update_from_json(house_world_machines_json()).expect("machine defs are valid DMML");
    let mut machines: HashMap<String, MachineBody> = HashMap::new();
    for batch in &update.batches {
        for m in &batch.machines {
            machines.insert(m.node.segments.join("/"), MachineBody { states: m.states.clone(), transitions: m.transitions.clone() });
        }
    }
    let machines = Arc::new(machines);

    let seed_commit = LoweredCommit {
        predicate_verb: "mints".to_string(),
        consumes: Vec::new(),
        produces: seed_state().into_iter().map(|(n, s)| Triple { subject: n.to_string(), predicate: "state".to_string(), object: TripleValue::Node(s.to_string()) }).collect(),
        refs: HashMap::new(),
    };
    let history = Arc::new(Mutex::new(vec![seed_commit]));
    let commit_log: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
    let last_seen: Arc<Mutex<HashMap<String, Materialized>>> = Arc::new(Mutex::new(HashMap::new()));

    let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind arena port");
    eprintln!("episode_arena listening on 127.0.0.1:{port}");

    for incoming in listener.incoming() {
        let Ok(stream) = incoming else { continue };
        let machines = Arc::clone(&machines);
        let history = Arc::clone(&history);
        let commit_log = Arc::clone(&commit_log);
        let last_seen = Arc::clone(&last_seen);
        std::thread::spawn(move || handle_connection(stream, machines, history, commit_log, last_seen));
    }
}
