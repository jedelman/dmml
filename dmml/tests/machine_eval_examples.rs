//! Independent verification of `dmml::machine::eval_exists`/`eval_guard`
//! against MACHINE_SPEC.md's "Evaluating EXISTS" worked examples --
//! written from the spec text, not derived from the implementation.

use dmml::interpret::Materialized;
use dmml::lower::{LoweredCommit, Triple, TripleValue};
use dmml::machine::{eval_exists, eval_guard, parse_machine_body, EvalContext};
use std::collections::HashMap;

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

/// The three facts every worked example in the spec's "Evaluating
/// EXISTS" section is checked against: `edge/12 state unlocked`,
/// `room/1 hasEdge edge/12`, `edge/12 connectsTo room/2`.
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

fn parse_first_guard(body: &str) -> dmml::machine::GuardClause {
    let parsed = parse_machine_body(body).expect("should parse");
    parsed.transitions[0].guards[0].clone()
}

#[test]
fn example_1_self_state_check_true() {
    let guard = parse_first_guard("transition t { guard: EXISTS(self state unlocked) }");
    let ctx = EvalContext {
        self_node: "edge/12".to_string(),
        params: HashMap::new(),
    };
    assert!(eval_exists(&guard.exists.pattern, &ctx, &world()));
    assert!(eval_guard(&guard, &ctx, &world()));
}

#[test]
fn example_2_self_state_check_false_for_different_edge() {
    let guard = parse_first_guard("transition t { guard: EXISTS(self state unlocked) }");
    let ctx = EvalContext {
        self_node: "edge/99".to_string(),
        params: HashMap::new(),
    };
    assert!(!eval_exists(&guard.exists.pattern, &ctx, &world()));
    assert!(!eval_guard(&guard, &ctx, &world()));
}

#[test]
fn example_3_multi_hop_traversal_with_existential_anchor() {
    let guard = parse_first_guard(
        "transition t { guard: EXISTS(?room hasEdge self connectsTo $dest) }",
    );
    let mut params = HashMap::new();
    params.insert("dest".to_string(), "room/2".to_string());
    let ctx = EvalContext {
        self_node: "edge/12".to_string(),
        params,
    };
    assert!(eval_exists(&guard.exists.pattern, &ctx, &world()));
    assert!(eval_guard(&guard, &ctx, &world()));
}

#[test]
fn example_3_fails_when_dest_does_not_match() {
    let guard = parse_first_guard(
        "transition t { guard: EXISTS(?room hasEdge self connectsTo $dest) }",
    );
    let mut params = HashMap::new();
    params.insert("dest".to_string(), "room/999".to_string());
    let ctx = EvalContext {
        self_node: "edge/12".to_string(),
        params,
    };
    assert!(!eval_exists(&guard.exists.pattern, &ctx, &world()));
}

#[test]
fn example_4_negated_guard_holds_on_empty_world() {
    let guard = parse_first_guard("transition t { guard: not EXISTS(guardPost/3 occupiedBy ?guard) }");
    let ctx = EvalContext::default();
    let empty = Materialized::default();
    assert!(!eval_exists(&guard.exists.pattern, &ctx, &empty));
    assert!(guard.negated);
    assert!(eval_guard(&guard, &ctx, &empty));
}

/// Not a worked example in prose form, but explicitly named in the
/// spec's "Not fully worked" section: a hop landing on a non-Node
/// value (e.g. a Number) must fail the walk, regardless of the hop's
/// own term shape.
#[test]
fn non_node_value_fails_the_walk() {
    let commit = LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![],
        produces: vec![fact("room/1", "dampness", TripleValue::Number("0.4".to_string()))],
        via: None,
        responds_to: None,
    };
    let world = Materialized::from_commits(&[commit]);

    let guard = parse_first_guard("transition t { guard: EXISTS(room/1 dampness ?x) }");
    let ctx = EvalContext::default();
    assert!(!eval_exists(&guard.exists.pattern, &ctx, &world));
}

/// Generalization: an unbound `$param` a guard references (never
/// supplied in `ctx.params`) makes the guard fail rather than panic or
/// error.
#[test]
fn unbound_param_fails_the_guard_without_panicking() {
    let guard = parse_first_guard("transition t { guard: EXISTS(self holds $item) }");
    let ctx = EvalContext {
        self_node: "player".to_string(),
        params: HashMap::new(),
    };
    assert!(!eval_exists(&guard.exists.pattern, &ctx, &Materialized::default()));
}
