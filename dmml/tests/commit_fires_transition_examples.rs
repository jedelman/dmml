//! Independent verification of `dmml::machine::commit_fires_transition`
//! against `MACHINE_SPEC.md`'s "Worked examples (wiring)" section --
//! the effects-matching half `may_fire` alone can't answer (see that
//! function's own doc comment for why the two are distinct questions).

use dmml::interpret::Materialized;
use dmml::lower::{ConsumeRef, FactRef, LoweredCommit, StrongRef, Triple, TripleValue};
use dmml::machine::{commit_fires_transition, Effect, EvalContext, FiresTransitionError};

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

/// Guard-satisfying world: `edge/12` is `locked`, player holds the key
/// -- the `unlock` transition's guards genuinely hold here.
fn world_before() -> Materialized {
    let commit = LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![],
        produces: vec![
            fact("edge/12", "state", node("locked")),
            fact("player", "holds", node("key/7")),
        ],
        via: None,
        responds_to: None,
    };
    Materialized::from_commits(&[commit])
}

/// A world where `edge/12` is already `unlocked` -- the implicit
/// `EXISTS(self state locked)` guard from `unlock`'s `from: locked`
/// fails here, regardless of what the candidate commit itself contains.
fn world_before_unlocked() -> Materialized {
    let commit = LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![],
        produces: vec![
            fact("edge/12", "state", node("unlocked")),
            fact("player", "holds", node("key/7")),
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

fn edge_12_ctx() -> EvalContext {
    EvalContext {
        self_node: "edge/12".to_string(),
        params: std::collections::HashMap::new(),
    }
}

fn empty_candidate() -> LoweredCommit {
    LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![],
        produces: vec![],
        via: None,
        responds_to: None,
    }
}

fn fact_ref_retracting_locked(object: Option<TripleValue>) -> FactRef {
    FactRef {
        commit: StrongRef {
            uri: "at://did:example/collection/1".to_string(),
            cid: "bafyExample".to_string(),
        },
        subject: "edge/12".to_string(),
        predicate: "state".to_string(),
        object,
    }
}

#[test]
fn guard_not_satisfied_when_state_is_not_locked() {
    let doc = dmml::parse(DOC_SRC).expect("should parse");
    let map = dmml::machine::parse_all_machines(&doc).expect("machine body should parse");
    let body = map.get("edge/12").expect("keyed by \"edge/12\"");

    let result = commit_fires_transition(
        body,
        "unlock",
        &edge_12_ctx(),
        &world_before_unlocked(),
        &empty_candidate(),
    );

    assert_eq!(result, Err(FiresTransitionError::GuardNotSatisfied));
}

#[test]
fn correct_candidate_matches_both_effects() {
    let doc = dmml::parse(DOC_SRC).expect("should parse");
    let map = dmml::machine::parse_all_machines(&doc).expect("machine body should parse");
    let body = map.get("edge/12").expect("keyed by \"edge/12\"");

    let candidate = LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![ConsumeRef::Fact(fact_ref_retracting_locked(Some(node("locked"))))],
        produces: vec![fact("edge/12", "state", node("unlocked"))],
        via: None,
        responds_to: None,
    };

    let result = commit_fires_transition(body, "unlock", &edge_12_ctx(), &world_before(), &candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn missing_assert_is_reported() {
    let doc = dmml::parse(DOC_SRC).expect("should parse");
    let map = dmml::machine::parse_all_machines(&doc).expect("machine body should parse");
    let body = map.get("edge/12").expect("keyed by \"edge/12\"");

    let candidate = LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![ConsumeRef::Fact(fact_ref_retracting_locked(Some(node("locked"))))],
        produces: vec![],
        via: None,
        responds_to: None,
    };

    let result = commit_fires_transition(body, "unlock", &edge_12_ctx(), &world_before(), &candidate);

    assert_eq!(
        result,
        Err(FiresTransitionError::EffectMismatch {
            missing: vec![Effect::Assert("unlocked".to_string())],
        })
    );
}

#[test]
fn wildcard_retract_still_satisfies_the_effect() {
    let doc = dmml::parse(DOC_SRC).expect("should parse");
    let map = dmml::machine::parse_all_machines(&doc).expect("machine body should parse");
    let body = map.get("edge/12").expect("keyed by \"edge/12\"");

    let candidate = LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![ConsumeRef::Fact(fact_ref_retracting_locked(None))],
        produces: vec![fact("edge/12", "state", node("unlocked"))],
        via: None,
        responds_to: None,
    };

    let result = commit_fires_transition(body, "unlock", &edge_12_ctx(), &world_before(), &candidate);

    assert_eq!(result, Ok(()));
}

/// A `ConsumeRef::Strong` never satisfies a `Retract` effect -- the
/// deliberate, documented fail-closed choice `commit_fires_transition`'s
/// own doc comment explains (unverifiable from a plain `Materialized`,
/// so rejected rather than assumed).
#[test]
fn strong_consume_never_satisfies_a_retract_effect() {
    let doc = dmml::parse(DOC_SRC).expect("should parse");
    let map = dmml::machine::parse_all_machines(&doc).expect("machine body should parse");
    let body = map.get("edge/12").expect("keyed by \"edge/12\"");

    let candidate = LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![ConsumeRef::Strong(StrongRef {
            uri: "at://did:example/collection/1".to_string(),
            cid: "bafyExample".to_string(),
        })],
        produces: vec![fact("edge/12", "state", node("unlocked"))],
        via: None,
        responds_to: None,
    };

    let result = commit_fires_transition(body, "unlock", &edge_12_ctx(), &world_before(), &candidate);

    assert_eq!(
        result,
        Err(FiresTransitionError::EffectMismatch {
            missing: vec![Effect::Retract("locked".to_string())],
        })
    );
}

#[test]
fn unknown_transition_ident() {
    let doc = dmml::parse(DOC_SRC).expect("should parse");
    let map = dmml::machine::parse_all_machines(&doc).expect("machine body should parse");
    let body = map.get("edge/12").expect("keyed by \"edge/12\"");

    let result = commit_fires_transition(
        body,
        "openSesame",
        &edge_12_ctx(),
        &world_before(),
        &empty_candidate(),
    );

    assert_eq!(result, Err(FiresTransitionError::UnknownTransition));
}
