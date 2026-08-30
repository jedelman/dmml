//! Independent verification of `dmml::machine::all_machines`/`may_fire`
//! against MACHINE_SPEC.md's "Worked examples (wiring)" section --
//! written from the spec text, not derived from the implementation.

use dmml::ast::{Document, TopLevelItem};
use dmml::from_json::{commit_from_json, machine_from_json};
use dmml::interpret::Materialized;
use dmml::lower::{LoweredCommit, Triple, TripleValue};
use dmml::machine::{all_machines, may_fire, EvalContext};

fn fact(subject: &str, predicate: &str, object: TripleValue) -> Triple {
    Triple {
        subject: subject.to_string(),
        predicate: predicate.to_string(),
        object,
    }
}

fn node(s: &str) -> TripleValue {
    TripleValue::Node(s.to_string())
}

fn world() -> Materialized {
    let commit = LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![],
        produces: vec![
            fact("edge/12", "state", node("unlocked")),
            fact("room/1", "hasEdge", node("edge/12")),
            fact("edge/12", "connectsTo", node("room/2")),
        ],
        refs: std::collections::HashMap::new(),
    };
    Materialized::from_commits(&[commit])
}

const DOC_JSON: &str = r#"{
    "node": "edge/12",
    "states": [{"ident": "locked"}, {"ident": "unlocked"}],
    "transitions": [
        {
            "ident": "unlock",
            "from": "locked",
            "to": "unlocked",
            "guards": [
                {"exists": {
                    "anchor": {"kind": "node", "value": "player"},
                    "hops": [{"predicate": "holds", "term": {"kind": "node", "value": "key/7"}}]
                }}
            ]
        }
    ]
}"#;

fn doc_with_edge_12_machine() -> Document {
    let stmt = machine_from_json(DOC_JSON).expect("machine JSON should build");
    Document {
        items: vec![TopLevelItem::Machine(stmt)],
    }
}

#[test]
fn example_1_all_machines_keys_by_joined_node_ref() {
    let doc = doc_with_edge_12_machine();
    let map = all_machines(&doc);
    assert_eq!(map.len(), 1);
    let body = map.get("edge/12").expect("keyed by \"edge/12\"");
    assert_eq!(body.states.len(), 2);
    assert_eq!(body.transitions.len(), 1);
    assert_eq!(body.transitions[0].ident, "unlock");
}

#[test]
fn example_2_may_fire_false_short_circuits_on_implicit_from_guard() {
    let doc = doc_with_edge_12_machine();
    let map = all_machines(&doc);
    let body = &map["edge/12"];

    let ctx = EvalContext {
        self_node: "edge/12".to_string(),
        params: Default::default(),
    };
    // world's edge/12 is already "unlocked", not "locked" -- the
    // implicit EXISTS(self state locked) guard from `from: locked`
    // fails, regardless of whether "player holds key/7" holds (it
    // doesn't, in this world either).
    assert_eq!(may_fire(body, "unlock", &ctx, &world()), Some(false));
}

#[test]
fn example_3_may_fire_none_for_undeclared_transition() {
    let doc = doc_with_edge_12_machine();
    let map = all_machines(&doc);
    let body = &map["edge/12"];

    let ctx = EvalContext {
        self_node: "edge/12".to_string(),
        params: Default::default(),
    };
    assert_eq!(may_fire(body, "openSesame", &ctx, &world()), None);
}

/// Not a worked example in the spec -- generalization: a document with
/// no `machine_stmt` items at all builds an empty map, not an error.
#[test]
fn no_machines_in_document_is_an_empty_map() {
    let commit = commit_from_json(
        r#"{"verb": "becomes", "declares": [{"kind": "relation", "name": "opensTo"}],
            "facts": [{"subject": "room/1", "predicate": "opensTo", "object": {"kind": "node", "value": "room/2"}}]}"#,
    )
    .expect("should build");
    let doc = Document {
        items: vec![TopLevelItem::Commit(commit)],
    };
    let map = all_machines(&doc);
    assert!(map.is_empty());
}
