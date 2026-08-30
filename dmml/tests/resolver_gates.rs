//! Truth-table tests for the five gate functions in `dmml::resolver`.
//! Four (`resolves`, `factref_matches`, `cross_repo_commit_valid`,
//! `commit_valid_despite_dangling_factref`) are each a direct
//! transcription of a Thermite contract that certified L3 (see
//! thermite-contracts/*.th) -- these tests don't re-prove anything, Forge
//! already did that, they pin that the Rust transcription didn't drift
//! from what was proven. `commit_is_valid` is the fifth and is NOT one of
//! those right now -- see its own doc comment in `resolver.rs` for why;
//! its test below is ordinary truth-table coverage of real (not proven)
//! Rust, same as any other function in this crate.

use dmml::resolver::{
    commit_is_valid, commit_valid_despite_dangling_factref, cross_repo_commit_valid,
    factref_matches, resolves,
};

#[test]
fn resolves_ignores_foreign_repo_accepted() {
    for foreign in [false, true] {
        assert!(resolves(true, foreign));
        assert!(!resolves(false, foreign));
    }
}

#[test]
fn commit_is_valid_requires_both_consumes_and_requires_to_be_valid() {
    assert!(commit_is_valid(true, true));
    assert!(!commit_is_valid(true, false));
    assert!(!commit_is_valid(false, true));
    assert!(!commit_is_valid(false, false));
}

#[test]
fn factref_matches_wildcard_and_equality() {
    assert!(factref_matches(false, false)); // no object specified -> wildcard
    assert!(factref_matches(false, true));
    assert!(factref_matches(true, true)); // object specified, equal
    assert!(!factref_matches(true, false)); // object specified, not equal
}

#[test]
fn cross_repo_commit_valid_fails_closed() {
    assert!(!cross_repo_commit_valid(true, true));
    assert!(!cross_repo_commit_valid(true, false));
    assert!(cross_repo_commit_valid(false, true));
    assert!(!cross_repo_commit_valid(false, false));
}

#[test]
fn dangling_factref_never_invalidates_carrying_commit() {
    for dangles in [false, true] {
        assert!(commit_valid_despite_dangling_factref(dangles, true));
        assert!(!commit_valid_despite_dangling_factref(dangles, false));
    }
}
