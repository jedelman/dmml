//! `interpret::requires_are_valid` -- the caller-side check
//! `resolver::commit_is_valid`'s own doc comment names: does every
//! `StrongRef` under a commit's `refs["requires"]` role actually resolve
//! against a real commit history.

use dmml::interpret::{requires_are_valid, IdentifiedCommit};
use dmml::lower::{LoweredCommit, StrongRef};
use std::collections::HashMap;

fn commit(cid: &str) -> IdentifiedCommit {
    IdentifiedCommit {
        uri: format!("at://did:plc:test/x/{cid}"),
        cid: cid.to_string(),
        commit: LoweredCommit {
            predicate_verb: "mints".to_string(),
            consumes: vec![],
            produces: vec![],
            refs: HashMap::new(),
        },
    }
}

fn requiring(cids: &[&str]) -> LoweredCommit {
    let mut refs = HashMap::new();
    if !cids.is_empty() {
        refs.insert(
            "requires".to_string(),
            cids.iter()
                .map(|cid| StrongRef {
                    uri: format!("at://did:plc:test/x/{cid}"),
                    cid: cid.to_string(),
                })
                .collect(),
        );
    }
    LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![],
        produces: vec![],
        refs,
    }
}

#[test]
fn a_commit_with_no_requires_role_at_all_is_vacuously_valid() {
    let history = vec![commit("only-commit")];
    assert!(requires_are_valid(&history, &requiring(&[])));
}

#[test]
fn a_single_requirement_that_resolves_is_valid() {
    let history = vec![commit("a"), commit("b")];
    assert!(requires_are_valid(&history, &requiring(&["a"])));
}

#[test]
fn a_single_requirement_that_does_not_resolve_is_invalid() {
    let history = vec![commit("a")];
    assert!(!requires_are_valid(&history, &requiring(&["does-not-exist"])));
}

#[test]
fn all_of_many_requirements_must_resolve() {
    let history = vec![commit("a"), commit("b"), commit("c")];
    assert!(requires_are_valid(&history, &requiring(&["a", "b", "c"])));
}

#[test]
fn one_missing_requirement_among_many_invalidates_the_whole_commit() {
    // This is the "index commit requires hundreds of others" shape: any
    // single missing link in that list should fail the whole check, not
    // be silently ignored because most of the list resolved fine.
    let history = vec![commit("a"), commit("b")];
    assert!(!requires_are_valid(&history, &requiring(&["a", "b", "missing"])));
}

#[test]
fn an_index_commits_own_validity_already_depends_on_every_member_it_requires() {
    // The "new patterns, not new primitives" shape from this feature's
    // own design: one index commit requires a whole group; nothing here
    // adds a second, group-shaped kind of reference for that -- the index
    // commit is just an ordinary commit whose OWN requires_are_valid
    // check already depends on every member it lists, the same as any
    // commit requiring anything else. "Importing the group" reduces to
    // "requiring the index commit," and the index commit's own validity
    // is exactly what guarantees the group actually resolved.
    let mut index = commit("index");
    index.commit.refs.insert(
        "requires".to_string(),
        vec![
            StrongRef { uri: "at://did:plc:test/x/member-a".to_string(), cid: "member-a".to_string() },
            StrongRef { uri: "at://did:plc:test/x/member-b".to_string(), cid: "member-b".to_string() },
        ],
    );

    let history_with_both_members = vec![index.clone(), commit("member-a"), commit("member-b")];
    assert!(requires_are_valid(&history_with_both_members, &index.commit));

    let history_missing_a_member = vec![index.clone(), commit("member-a")];
    assert!(!requires_are_valid(&history_missing_a_member, &index.commit));
}
