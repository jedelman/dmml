//! `interpret::ancestors_of` -- the backward-walk counterpart to
//! `reachable_from`'s forward walk, needed to materialize a linear
//! player-commit chain "as of" a past point (`since`/`at`) rather than
//! its current head.

use dmml::interpret::{ancestors_of, IdentifiedCommit};
use dmml::lower::{LoweredCommit, StrongRef};

fn commit(cid: &str, responds_to: Option<&str>) -> IdentifiedCommit {
    let mut refs = std::collections::HashMap::new();
    if let Some(cid) = responds_to {
        refs.insert(
            "respondsTo".to_string(),
            vec![StrongRef {
                uri: format!("at://did:plc:test/x/{cid}"),
                cid: cid.to_string(),
            }],
        );
    }
    IdentifiedCommit {
        uri: format!("at://did:plc:test/x/{cid}"),
        cid: cid.to_string(),
        commit: LoweredCommit {
            predicate_verb: "mints".to_string(),
            consumes: vec![],
            produces: vec![],
            refs,
        },
    }
}

fn cids(commits: &[IdentifiedCommit]) -> Vec<&str> {
    commits.iter().map(|c| c.cid.as_str()).collect()
}

#[test]
fn ancestors_of_the_root_is_just_the_root() {
    let chain = vec![commit("genesis", None)];
    assert_eq!(cids(&ancestors_of(&chain, "genesis")), vec!["genesis"]);
}

#[test]
fn ancestors_of_a_middle_commit_excludes_what_comes_after_it() {
    let chain = vec![
        commit("genesis", None),
        commit("c1", Some("genesis")),
        commit("c2", Some("c1")),
    ];
    assert_eq!(
        cids(&ancestors_of(&chain, "c1")),
        vec!["genesis", "c1"],
        "materializing as of c1 must not see c2, which comes after it"
    );
}

#[test]
fn ancestors_of_the_head_is_the_whole_chain_genesis_first() {
    let chain = vec![
        commit("genesis", None),
        commit("c1", Some("genesis")),
        commit("c2", Some("c1")),
    ];
    assert_eq!(cids(&ancestors_of(&chain, "c2")), vec!["genesis", "c1", "c2"]);
}

#[test]
fn unknown_cid_returns_nothing() {
    let chain = vec![commit("genesis", None)];
    assert_eq!(ancestors_of(&chain, "does-not-exist"), Vec::<IdentifiedCommit>::new());
}

#[test]
fn a_cyclic_responds_to_stops_rather_than_looping_forever() {
    // c1 responds to c2 and c2 responds to c1 -- a malformed or
    // adversarial chain (the player's own PDS is sovereign, §7; nothing
    // stops them from writing this, whether by bug or on purpose).
    let chain = vec![commit("c1", Some("c2")), commit("c2", Some("c1"))];
    let result = ancestors_of(&chain, "c1");
    assert!(
        result.len() <= 2,
        "must terminate rather than looping forever on a cycle; got {} entries",
        result.len()
    );
}

#[test]
fn a_dangling_responds_to_stops_the_walk_there_rather_than_panicking() {
    // c1's own parent ("missing") was never included in `chain` -- a
    // caller passed a partial/scoped slice, not a full history.
    let chain = vec![commit("c1", Some("missing")), commit("c2", Some("c1"))];
    assert_eq!(
        cids(&ancestors_of(&chain, "c2")),
        vec!["c1", "c2"],
        "a dangling responds_to should stop the walk there, not panic or \
         silently invent a root"
    );
}
