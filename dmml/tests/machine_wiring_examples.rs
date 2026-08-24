//! Independent verification of `dmml::machine::parse_all_machines`/
//! `may_fire` against MACHINE_SPEC.md's "Worked examples (wiring)"
//! section -- written from the spec text, not derived from the
//! implementation.

use dmml::interpret::Materialized;
use dmml::lower::{LoweredCommit, Triple, TripleValue};
use dmml::machine::{may_fire, parse_all_machines, EvalContext};

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
        via: None,
        responds_to: None,
    };
    Materialized::from_commits(&[commit])
}

const DOC_SRC: &str = "
machine edge/12 {
  state locked
  state unlocked

  transition unlock {
    from: locked
    to: unlocked
    guard: EXISTS(player holds key/7)
  }
}
";

#[test]
fn example_1_parse_all_machines_keys_by_joined_node_ref() {
    let doc = dmml::parse(DOC_SRC).expect("should parse");
    let map = parse_all_machines(&doc).expect("machine body should parse");
    assert_eq!(map.len(), 1);
    let body = map.get("edge/12").expect("keyed by \"edge/12\"");
    assert_eq!(body.states.len(), 2);
    assert_eq!(body.transitions.len(), 1);
    assert_eq!(body.transitions[0].ident, "unlock");
}

#[test]
fn example_2_may_fire_false_short_circuits_on_implicit_from_guard() {
    let doc = dmml::parse(DOC_SRC).expect("should parse");
    let map = parse_all_machines(&doc).expect("machine body should parse");
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
    let doc = dmml::parse(DOC_SRC).expect("should parse");
    let map = parse_all_machines(&doc).expect("machine body should parse");
    let body = &map["edge/12"];

    let ctx = EvalContext {
        self_node: "edge/12".to_string(),
        params: Default::default(),
    };
    assert_eq!(may_fire(body, "openSesame", &ctx, &world()), None);
}

/// Not a worked example in the spec -- generalization: a document with
/// no `machine_stmt` items at all parses to an empty map, not an error.
#[test]
fn no_machines_in_document_is_an_empty_map() {
    let doc = dmml::parse("commit becomes {\n  declare relation opensTo\n  room/1 opensTo room/2\n}\n")
        .expect("should parse");
    let map = parse_all_machines(&doc).expect("no machines to fail on");
    assert!(map.is_empty());
}

/// Generalization: a malformed machine body surfaces its own node as
/// the error key, not a generic failure.
#[test]
fn malformed_machine_body_reports_its_own_node_as_the_error_key() {
    let src = "
machine door/9 {
  transition noop { }
}
";
    let doc = dmml::parse(src).expect("should parse at the document level");
    let err = parse_all_machines(&doc).expect_err("noop transition should be rejected");
    assert_eq!(err.0, "door/9");
}
