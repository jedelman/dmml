//! Companion to `valar_operate_test.py`: the OPERATE tier, checked for
//! real. Takes a JSON file `{"node": ..., "transition": ..., "params":
//! {...} | null}` -- a model's structurally-bounded choice of ONE
//! action from the real transition catalog (a `oneOf` of const-tagged
//! branches, one per actual `valinor_house.rs` transition; nothing
//! outside that list is representable in the schema at all) -- and
//! fires it for real against the seed state of that exact world.
//!
//! Unlike `valinor.rs`/`door.rs`/etc., this doesn't hand-author the
//! commit that fires the transition -- it builds one generically from
//! whatever (node, transition, params) the input names, the same way a
//! real dispatcher would have to: look up the transition's declared
//! `from`/`to` (if any) to build the state-change facts, look up its
//! params to build the param-value facts, and call `commit_fires_
//! transition` exactly as every other example does.
//!
//! Usage: `cargo run -p dmml --example operate_check -- path/to/choice.json`

use dmml::from_json::update_from_json;
use dmml::interpret::Materialized;
use dmml::lower::{lower_commit, LoweredCommit};
use dmml::machine::{self, EvalContext, MachineBody};
use serde::Deserialize;
use std::collections::HashMap;

const SEED_URI: &str = "at://did:example:valinor/world.valinor/operate-genesis";
const SEED_CID: &str = "operate-seed-cid-0";

#[derive(Deserialize)]
struct Choice {
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

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: operate_check <path-to-choice.json>");
        std::process::exit(2);
    });
    let choice_json = std::fs::read_to_string(&path).expect("read choice file");
    let choice: Choice = serde_json::from_str(&choice_json).expect("parse choice JSON");

    let seed_update = update_from_json(seed_json()).expect("seed JSON is valid DMML");
    let mut machines: HashMap<String, MachineBody> = HashMap::new();
    for batch in &seed_update.batches {
        for m in &batch.machines {
            machines.insert(m.node.segments.join("/"), MachineBody { states: m.states.clone(), transitions: m.transitions.clone() });
        }
    }
    let history: Vec<LoweredCommit> = seed_update.batches.iter().flat_map(|b| b.commits.iter().map(lower_commit)).collect();
    let world_before = Materialized::from_commits(&history);

    let Some(body) = machines.get(&choice.node) else {
        println!("FAIL: '{}' is not a real machine node in this world.", choice.node);
        std::process::exit(1);
    };
    let Some(decl) = body.transitions.iter().find(|t| t.ident == choice.transition) else {
        println!("FAIL: '{}' has no transition named '{}'.", choice.node, choice.transition);
        std::process::exit(1);
    };

    let params = choice.params.unwrap_or_default();
    let declared_params: Vec<&String> = decl.params.iter().collect();
    let missing: Vec<&&String> = declared_params.iter().filter(|p| !params.contains_key(p.as_str())).collect();
    if !missing.is_empty() {
        println!("FAIL: transition '{}' requires params {:?}, missing: {:?}", choice.transition, declared_params, missing);
        std::process::exit(1);
    }

    // Build a candidate commit generically from the transition's own
    // declared from/to and params -- exactly what a real dispatcher
    // would have to synthesize, not hand-authored per example.
    let mut facts_json = Vec::new();
    if let (Some(from), Some(to)) = (&decl.from, &decl.to) {
        facts_json.push(format!(
            r#"{{"subject": "{}", "predicate": "state", "object": {{"kind": "node", "value": "{to}"}}}}"#,
            choice.node
        ));
        let _ = from; // from is used only in `consumes` below
    }
    for (name, value) in &params {
        facts_json.push(format!(
            r#"{{"subject": "{}", "predicate": "{name}", "object": {{"kind": "node", "value": "{value}"}}}}"#,
            choice.node
        ));
    }
    let declares_json: Vec<String> = params.keys().map(|k| format!(r#"{{"kind": "attribute", "name": "{k}"}}"#)).collect();

    let consumes_json = if let Some(from) = &decl.from {
        format!(
            r#"[{{"kind": "fact", "commit": {{"uri": "{SEED_URI}", "cid": "{SEED_CID}"}}, "subject": "{}", "predicate": "state", "object": {{"kind": "node", "value": "{from}"}}}}]"#,
            choice.node
        )
    } else {
        "[]".to_string()
    };

    let commit_json = format!(
        r#"{{"update": [{{"commits": [{{"verb": "{}", "declares": [{}], "consumes": {consumes_json}, "facts": [{}]}}]}}]}}"#,
        choice.transition,
        declares_json.join(", "),
        facts_json.join(", "),
    );

    let update = match update_from_json(&commit_json) {
        Ok(u) => u,
        Err(e) => {
            println!("FAIL: could not build a real commit from this choice: {e}");
            std::process::exit(1);
        }
    };
    let candidate = lower_commit(&update.batches[0].commits[0]);

    let ctx = EvalContext { self_node: choice.node.clone(), params: params.clone() };
    match machine::commit_fires_transition(body, &choice.transition, &ctx, &world_before, &candidate) {
        Ok(()) => {
            println!(
                "PASS: '{}' firing '{}' with params {:?} is a real, legitimate action against the seed world.",
                choice.node, choice.transition, params
            );
        }
        Err(e) => {
            println!("FAIL: '{}' cannot legitimately fire '{}' right now: {e:?}", choice.node, choice.transition);
            std::process::exit(1);
        }
    }
}
