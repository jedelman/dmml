//! Independent verification of `dmml::validate::validate_same_repo_
//! consumes`/`commit_is_valid` against VALIDATION_SPEC.md's "Same-repo
//! consumes structural validation" section -- all 5 worked examples,
//! plus the not-fully-worked case (a cross-repo consume must void the
//! whole commit even when declarations_ok is true).

use dmml::lower::{ConsumeRef, FactRef, LoweredCommit, StrongRef};
use dmml::validate::{commit_is_valid, validate_same_repo_consumes, CrossRepoConsume};

const AUTHORING_DID: &str = "did:plc:aaaa1111";

fn empty_commit(consumes: Vec<ConsumeRef>) -> LoweredCommit {
    LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes,
        produces: vec![],
        refs: std::collections::HashMap::new(),
    }
}

fn strong(uri: &str, cid: &str) -> ConsumeRef {
    ConsumeRef::Strong(StrongRef {
        uri: uri.to_string(),
        cid: cid.to_string(),
    })
}

fn fact(commit_uri: &str, commit_cid: &str) -> ConsumeRef {
    ConsumeRef::Fact(FactRef {
        commit: StrongRef {
            uri: commit_uri.to_string(),
            cid: commit_cid.to_string(),
        },
        subject: "room/42".to_string(),
        predicate: "locked".to_string(),
        object: None,
    })
}

#[test]
fn example_1_same_repo_strong_no_violation() {
    let commit = empty_commit(vec![strong(
        "at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789",
        "bafyabcxyz",
    )]);
    assert_eq!(validate_same_repo_consumes(&commit, AUTHORING_DID), Ok(()));
}

#[test]
fn example_2_cross_repo_strong_one_violation() {
    let commit = empty_commit(vec![strong(
        "at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456",
        "bafyqrs456",
    )]);
    assert_eq!(
        validate_same_repo_consumes(&commit, AUTHORING_DID),
        Err(vec![CrossRepoConsume {
            index: 0,
            foreign_did: "did:plc:zzzz9999".to_string(),
        }])
    );
}

#[test]
fn example_3_cross_repo_fact_checks_commit_uri_not_subject_predicate() {
    let commit = empty_commit(vec![fact(
        "at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456",
        "bafyqrs456",
    )]);
    assert_eq!(
        validate_same_repo_consumes(&commit, AUTHORING_DID),
        Err(vec![CrossRepoConsume {
            index: 0,
            foreign_did: "did:plc:zzzz9999".to_string(),
        }])
    );
}

#[test]
fn example_4_mixed_only_violating_indices_in_order() {
    let commit = empty_commit(vec![
        strong(
            "at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/one",
            "bafyone",
        ),
        strong(
            "at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/two",
            "bafytwo",
        ),
        strong(
            "at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/three",
            "bafythree",
        ),
        fact(
            "at://did:plc:cccc3333/org.jason-edelman.writtenworld.commit/four",
            "bafyfour",
        ),
    ]);
    assert_eq!(
        validate_same_repo_consumes(&commit, AUTHORING_DID),
        Err(vec![
            CrossRepoConsume {
                index: 1,
                foreign_did: "did:plc:zzzz9999".to_string(),
            },
            CrossRepoConsume {
                index: 3,
                foreign_did: "did:plc:cccc3333".to_string(),
            },
        ])
    );
}

#[test]
fn example_5_empty_consumes_no_violations() {
    let commit = empty_commit(vec![]);
    assert_eq!(validate_same_repo_consumes(&commit, AUTHORING_DID), Ok(()));
}

/// Not a worked example -- only stated as rule 5's own language.
/// A cross-repo consume must void the whole commit even when
/// declarations_ok is true (matches cross_repo_consume_fails_closed.th).
#[test]
fn commit_is_valid_fails_closed_on_cross_repo_consume_regardless_of_declarations() {
    let commit = empty_commit(vec![strong(
        "at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456",
        "bafyqrs456",
    )]);
    assert!(!commit_is_valid(&commit, AUTHORING_DID, true));
}

#[test]
fn commit_is_valid_true_when_same_repo_and_declarations_ok() {
    let commit = empty_commit(vec![strong(
        "at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789",
        "bafyabcxyz",
    )]);
    assert!(commit_is_valid(&commit, AUTHORING_DID, true));
}
