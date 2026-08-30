//! Independent verification of `dmml::interpret::Materialized::
//! from_identified_commits` against MATERIALIZATION_SPEC.md's worked
//! examples -- written from the spec text, not derived from the
//! implementation.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::lower::{ConsumeRef, FactRef, LoweredCommit, StrongRef, Triple, TripleValue};

fn empty_commit() -> LoweredCommit {
    LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![],
        produces: vec![],
        refs: std::collections::HashMap::new(),
    }
}

fn mint() -> IdentifiedCommit {
    let mut commit = empty_commit();
    commit.produces.push(Triple {
        subject: "room/42".to_string(),
        predicate: "locked".to_string(),
        object: TripleValue::Boolean(true),
    });
    IdentifiedCommit {
        uri: "at://did:plc:aaaa/collection/mint".to_string(),
        cid: "bafymint".to_string(),
        commit,
    }
}

fn mint_ref() -> StrongRef {
    StrongRef {
        uri: "at://did:plc:aaaa/collection/mint".to_string(),
        cid: "bafymint".to_string(),
    }
}

#[test]
fn example_1_strong_consume_retracts_everything_the_target_produced() {
    let mut commit = empty_commit();
    commit.consumes.push(ConsumeRef::Strong(mint_ref()));
    let update = IdentifiedCommit {
        uri: "at://did:plc:aaaa/collection/update".to_string(),
        cid: "bafyupdate".to_string(),
        commit,
    };

    let world = Materialized::from_identified_commits(&[mint(), update]);
    assert_eq!(world.current_value("room/42", "locked"), None);
}

#[test]
fn example_2_fact_consume_with_matching_object_retracts_just_that_pair() {
    let mut commit = empty_commit();
    commit.consumes.push(ConsumeRef::Fact(FactRef {
        commit: mint_ref(),
        subject: "room/42".to_string(),
        predicate: "locked".to_string(),
        object: Some(TripleValue::Boolean(true)),
    }));
    let update = IdentifiedCommit {
        uri: "at://did:plc:aaaa/collection/update".to_string(),
        cid: "bafyupdate".to_string(),
        commit,
    };

    let world = Materialized::from_identified_commits(&[mint(), update]);
    assert_eq!(world.current_value("room/42", "locked"), None);
}

#[test]
fn example_3_fact_consume_with_non_matching_object_retracts_nothing() {
    let mut commit = empty_commit();
    commit.consumes.push(ConsumeRef::Fact(FactRef {
        commit: mint_ref(),
        subject: "room/42".to_string(),
        predicate: "locked".to_string(),
        object: Some(TripleValue::Boolean(false)),
    }));
    let update = IdentifiedCommit {
        uri: "at://did:plc:aaaa/collection/update".to_string(),
        cid: "bafyupdate".to_string(),
        commit,
    };

    let world = Materialized::from_identified_commits(&[mint(), update]);
    assert_eq!(
        world.current_value("room/42", "locked"),
        Some(&TripleValue::Boolean(true))
    );
}

#[test]
fn example_4_dangling_factref_fails_open() {
    let mut commit = empty_commit();
    commit.consumes.push(ConsumeRef::Fact(FactRef {
        commit: StrongRef {
            uri: "at://did:plc:aaaa/collection/nonexistent".to_string(),
            cid: "bafynope".to_string(),
        },
        subject: "room/42".to_string(),
        predicate: "locked".to_string(),
        object: None,
    }));
    let update = IdentifiedCommit {
        uri: "at://did:plc:aaaa/collection/update".to_string(),
        cid: "bafyupdate".to_string(),
        commit,
    };

    let world = Materialized::from_identified_commits(&[mint(), update]);
    assert_eq!(
        world.current_value("room/42", "locked"),
        Some(&TripleValue::Boolean(true))
    );
}

#[test]
fn example_5_retract_then_reassert_in_same_commit_is_a_net_update() {
    let mut commit = empty_commit();
    commit.consumes.push(ConsumeRef::Fact(FactRef {
        commit: mint_ref(),
        subject: "room/42".to_string(),
        predicate: "locked".to_string(),
        object: None,
    }));
    commit.produces.push(Triple {
        subject: "room/42".to_string(),
        predicate: "locked".to_string(),
        object: TripleValue::Boolean(false),
    });
    let combined = IdentifiedCommit {
        uri: "at://did:plc:aaaa/collection/combined".to_string(),
        cid: "bafycombined".to_string(),
        commit,
    };

    let world = Materialized::from_identified_commits(&[mint(), combined]);
    assert_eq!(
        world.current_value("room/42", "locked"),
        Some(&TripleValue::Boolean(false))
    );
}

/// Not a worked example -- generalization: a Strong consume targeting a
/// commit whose produces is empty retracts nothing (zero iterations),
/// not an error.
#[test]
fn strong_consume_of_an_empty_producer_retracts_nothing() {
    let pure_retraction = IdentifiedCommit {
        uri: "at://did:plc:aaaa/collection/pure".to_string(),
        cid: "bafypure".to_string(),
        commit: empty_commit(),
    };
    let mut commit = empty_commit();
    commit.consumes.push(ConsumeRef::Strong(StrongRef {
        uri: "at://did:plc:aaaa/collection/pure".to_string(),
        cid: "bafypure".to_string(),
    }));
    let consumer = IdentifiedCommit {
        uri: "at://did:plc:aaaa/collection/consumer".to_string(),
        cid: "bafyconsumer".to_string(),
        commit,
    };

    let world = Materialized::from_identified_commits(&[mint(), pure_retraction, consumer]);
    // mint's own triple is untouched -- nothing here targeted it.
    assert_eq!(
        world.current_value("room/42", "locked"),
        Some(&TripleValue::Boolean(true))
    );
}

/// Generalization: from_commits (the original produces-only fold)
/// stays available unchanged for callers with no consumes to worry
/// about.
#[test]
fn from_commits_is_unaffected_by_the_new_fold() {
    let commit = mint().commit;
    let world = dmml::interpret::Materialized::from_commits(&[commit]);
    assert_eq!(
        world.current_value("room/42", "locked"),
        Some(&TripleValue::Boolean(true))
    );
}
