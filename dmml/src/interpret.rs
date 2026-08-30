//! The resolver's actual fold, over real data: `SPEC.md` SS5, "assembles
//! the current materialized view by ... unioning the asserted content of
//! relevant, valid commits" -- applied to `crate::lower::LoweredCommit`
//! (real `Triple`s, not the abstract `u64` fact ids `crate::resolver`
//! models the frame property with).
//!
//! Two folds: `from_commits` is the original `produces`-only
//! materialization (last-write-wins per `(subject, predicate)`, no
//! notion of retraction at all -- kept as-is for callers with no
//! `consumes` to worry about). `from_identified_commits` is the real
//! one, per `MATERIALIZATION_SPEC.md` (issue #70): `consumes`-driven
//! retraction, reusing `crate::resolver::factref_matches` (already
//! Thermite-proven) rather than reimplementing its matching logic.

use crate::lower::{ConsumeRef, LoweredCommit, TripleValue};
use std::collections::HashMap;

/// A `LoweredCommit` paired with the stable identity a `ConsumeRef`
/// references it by. `LoweredCommit` itself carries none -- `uri` is
/// chosen by whoever publishes a commit (an `at://did/collection/rkey`),
/// not derivable from its content, same reasoning `dmml-substrate-kit`'s
/// `atproto_cid::TripleRef` already applies to pair a triple's content
/// hash with an externally-supplied `owner_did` rather than baking it
/// into the hash (CID computation itself lives in that separate crate,
/// not here -- this crate only ever carries an opaque `cid: String`).
#[derive(Debug, Clone, PartialEq)]
pub struct IdentifiedCommit {
    pub uri: String,
    pub cid: String,
    pub commit: LoweredCommit,
}

/// The current materialized view of a commit log's `produces` content:
/// for each `(subject, predicate)` pair ever asserted, the value from the
/// LAST commit in the log that asserted it. Matches the real engine's
/// own `current_value` semantics ("the latest value this specific
/// predicate on this specific node was asserted to have"), restricted
/// to what a flat `produces`-only fold can answer -- no `consumes`-driven
/// retraction (see module doc).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Materialized {
    current: HashMap<(String, String), TripleValue>,
}

impl Materialized {
    /// Folds `commits` in order into a fresh `Materialized` view. Later
    /// commits' assertions for the same `(subject, predicate)` overwrite
    /// earlier ones; a commit whose `produces` never mentions a given
    /// `(subject, predicate)` leaves any earlier value for it untouched.
    pub fn from_commits(commits: &[LoweredCommit]) -> Self {
        let mut current = HashMap::new();
        for commit in commits {
            for triple in &commit.produces {
                current.insert(
                    (triple.subject.clone(), triple.predicate.clone()),
                    triple.object.clone(),
                );
            }
        }
        Materialized { current }
    }

    /// The current value for `(subject, predicate)`, if anything in the
    /// folded log ever asserted one.
    pub fn current_value(&self, subject: &str, predicate: &str) -> Option<&TripleValue> {
        self.current
            .get(&(subject.to_string(), predicate.to_string()))
    }

    /// How many distinct `(subject, predicate)` pairs currently hold a
    /// value.
    pub fn len(&self) -> usize {
        self.current.len()
    }

    pub fn is_empty(&self) -> bool {
        self.current.is_empty()
    }

    /// Every distinct subject that currently has at least one asserted
    /// `(subject, predicate)` pair -- the search space `dmml::machine`'s
    /// `EXISTS` evaluator scans when a pattern's anchor is an
    /// existentially-bound `?var` (no known starting node to walk
    /// forward from). May yield the same subject more than once (once
    /// per predicate it has); callers that need a probe *set* rather
    /// than a probe *list* should dedupe.
    pub fn subjects(&self) -> impl Iterator<Item = &str> {
        self.current.keys().map(|(subject, _)| subject.as_str())
    }

    /// Every currently-held `(subject, predicate, value)` triple -- the
    /// whole materialized view, for a caller that needs to render or
    /// enumerate it (the Perceive route, #79: formats every triple as a
    /// plain-text fact line for the perception LLM call), not just probe
    /// one `(subject, predicate)` pair or list subjects.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str, &TripleValue)> {
        self.current
            .iter()
            .map(|((subject, predicate), value)| (subject.as_str(), predicate.as_str(), value))
    }

    /// The real fold, per `MATERIALIZATION_SPEC.md`: walks `commits` in
    /// order, applying each commit's `consumes` (retraction) before its
    /// `produces` (assertion) -- so a commit that both retracts an old
    /// value and asserts its replacement behaves as one atomic update,
    /// matching `resolver::WorldState::apply_combined_commit`'s own
    /// retract-then-assert convention.
    pub fn from_identified_commits(commits: &[IdentifiedCommit]) -> Self {
        let mut current: HashMap<(String, String), TripleValue> = HashMap::new();

        for identified in commits {
            for consume in &identified.commit.consumes {
                apply_consume(consume, commits, &mut current);
            }
            for triple in &identified.commit.produces {
                current.insert(
                    (triple.subject.clone(), triple.predicate.clone()),
                    triple.object.clone(),
                );
            }
        }

        Materialized { current }
    }
}

/// One differing fact between two materialized snapshots -- the primitive a
/// drift/staleness check needs (comparing "what a player remembered perceiving"
/// against "what's true now"). `Materialized::current_value` alone cannot answer
/// this: it only ever looks at one snapshot, so a caller cannot detect that the
/// same `(subject, predicate)` now holds a *different* value, or that a fact
/// existed previously but has since been retracted into `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct Divergence {
    pub subject: String,
    pub predicate: String,
    pub before: Option<TripleValue>,
    pub after: Option<TripleValue>,
}

/// Every `(subject, predicate)` pair whose value differs between two
/// materialized snapshots, sorted lexicographically by `(subject, predicate)`
/// for deterministic test assertions.
///
/// Walks the union of both sides' `(subject, predicate)` pairs rather than
/// just one side's -- a fact can appear in only `after` (something new came
/// into existence) or only `before` (something was retracted with nothing
/// replacing it), and both are real divergences, not just changed values.
/// Using only `before.iter()` would miss creations; using only `after.iter()`
/// would miss retractions.
pub fn diverges(before: &Materialized, after: &Materialized) -> Vec<Divergence> {
    let mut pairs: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    for (subject, predicate, _) in before.iter() {
        pairs.insert((subject.to_string(), predicate.to_string()));
    }
    for (subject, predicate, _) in after.iter() {
        pairs.insert((subject.to_string(), predicate.to_string()));
    }

    let mut result: Vec<Divergence> = pairs
        .into_iter()
        .filter_map(|(subject, predicate)| {
            let b = before.current_value(&subject, &predicate).cloned();
            let a = after.current_value(&subject, &predicate).cloned();
            if b == a {
                None
            } else {
                Some(Divergence {
                    subject,
                    predicate,
                    before: b,
                    after: a,
                })
            }
        })
        .collect();

    result.sort_by(|x, y| (&x.subject, &x.predicate).cmp(&(&y.subject, &y.predicate)));
    result
}

/// Walk backward from `at_cid` to collect its complete ancestor chain -- `at_cid` itself
/// and every commit it `responds_to`, recursively, back to (and including) the root.
///
/// This is the inverse direction of [`reachable_from`]: that function walks *forward*
/// from a genesis to collect everything built on top of it (the entire world from that
/// root forward), while this walks *backward* to materialize "the world exactly as it
/// stood at `at_cid`" -- a linear chain of history, not everything that came after.
/// Used by Perceive route handlers with `since` or `at` query parameters to materialize
/// a player-commit chain as of a specific past point, without including commits that
/// arrived later.
///
/// Returns genesis-first order (oldest ancestor first, `at_cid` last) so the result
/// can be passed directly to `Materialized::from_identified_commits`, which applies
/// commits in slice order. Returns empty if `at_cid` is unknown -- callers passing
/// an unrecognized CID get nothing, consistent with this file's fail-closed handling
/// of dangling references (see `apply_consume`).
///
/// Stops if a `cid` is revisited, rather than looping forever. `commits` comes from
/// a player's own PDS -- a repo they sovereignly control (`SPEC.md` §7) and could in
/// principle write a cyclic `responds_to` into, whether by a bug or on purpose; this
/// function has no way to validate the chain's shape ahead of time, so it treats a
/// repeat the same way it already treats a dangling reference -- stop there, return
/// what was collected so far, never hang.
pub fn ancestors_of(commits: &[IdentifiedCommit], at_cid: &str) -> Vec<IdentifiedCommit> {
    let by_cid: std::collections::HashMap<&str, &IdentifiedCommit> =
        commits.iter().map(|c| (c.cid.as_str(), c)).collect();

    let mut result = Vec::new();
    let mut visited: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut current_cid = at_cid;

    while let Some(commit) = by_cid.get(current_cid) {
        if !visited.insert(current_cid) {
            break;
        }
        result.push((*commit).clone());
        match commit.commit.refs.get("respondsTo").and_then(|v| v.first()) {
            Some(r) => current_cid = &r.cid,
            None => break,
        }
    }

    result.reverse();
    result
}

/// Scopes `commits` to "this world": `root_cid` itself, plus every
/// commit reachable by walking `respondsTo` backward to it (a commit
/// responds to the root, or responds to something that (transitively)
/// responds to the root). A real fix for a real gap found running
/// `client/examples/real_pds_loop.rs` against a live PDS
/// (2026-08-18): `com.atproto.repo.listRecords` returns every record a
/// DID ever wrote to a collection, forever -- nothing narrower than
/// that exists at the protocol level, so a caller has to draw this
/// boundary itself before treating a fetched set as "the current
/// world." Reuses `respondsTo` rather than inventing a new field:
/// unlike `via` (already a distinct, established meaning -- "the
/// operation or grant that authorized this commit," `SPEC.md` SS10),
/// `respondsTo` already means "this commit is a continuation of that
/// one," which a world's own commit chain legitimately is.
///
/// Every commit's own author is responsible for actually setting
/// `respondsTo` to form a connected chain back to a genesis commit --
/// this function only walks the links it's given; a commit written
/// without a `respondsTo` chain back to `root_cid` is correctly
/// excluded, not an error in this function.
///
/// Datalog-backed as of the cutover that added `crate::
/// datalog_reachability` -- a real transitive-closure fixpoint (crepe)
/// replaced the hand-rolled "fixed-point iteration... simpler than a
/// proper graph walk" this doc comment used to describe here, proven
/// equivalent by that module's own tests (including a cycle-inside-the-
/// reachable-set case this function had never actually had a test for).
/// Kept as a stable, named function since it's part of this crate's
/// public API even though nothing in this repo currently calls it
/// outside its own module doc references.
pub fn reachable_from(commits: &[IdentifiedCommit], root_cid: &str) -> Vec<IdentifiedCommit> {
    crate::datalog_reachability::reachable_from(commits, root_cid)
}

/// The `requires_are_valid` input `resolver::commit_is_valid`'s own doc
/// comment names as a caller's job: whether every `StrongRef` under
/// `commit.refs["requires"]` actually resolves to a real commit somewhere
/// in `history`. A commit with no `requires` role at all vacuously
/// satisfies this (nothing to check), matching how an empty `consumes`
/// list is likewise never a validity problem elsewhere in this crate.
/// Resolution here means "present by `(uri, cid)`" -- the same identity
/// `apply_consume` above already uses to find a `ConsumeRef`'s target,
/// not a stronger claim about the required commit's own content being
/// well-formed (that commit's own validity was, or will be, checked when
/// IT was resolved).
pub fn requires_are_valid(history: &[IdentifiedCommit], commit: &LoweredCommit) -> bool {
    let Some(required) = commit.refs.get("requires") else {
        return true;
    };
    required
        .iter()
        .all(|r| history.iter().any(|c| c.uri == r.uri && c.cid == r.cid))
}

/// Applies one `ConsumeRef` against the running fold state, per
/// `MATERIALIZATION_SPEC.md`'s "Apply consumes" step. A dangling
/// reference (target commit not found, or -- for `Fact` -- the target
/// never produced that `(subject, predicate)`) is a no-op, never an
/// error: matches `resolver::commit_valid_despite_dangling_factref`'s
/// fails-open posture.
fn apply_consume(
    consume: &ConsumeRef,
    commits: &[IdentifiedCommit],
    current: &mut HashMap<(String, String), TripleValue>,
) {
    let target_ref = match consume {
        ConsumeRef::Strong(sr) => sr,
        ConsumeRef::Fact(fr) => &fr.commit,
    };
    let Some(target) = commits
        .iter()
        .find(|c| c.uri == target_ref.uri && c.cid == target_ref.cid)
    else {
        return;
    };

    match consume {
        ConsumeRef::Strong(_) => {
            for triple in &target.commit.produces {
                current.remove(&(triple.subject.clone(), triple.predicate.clone()));
            }
        }
        ConsumeRef::Fact(fr) => {
            let Some(actual) = target
                .commit
                .produces
                .iter()
                .find(|t| t.subject == fr.subject && t.predicate == fr.predicate)
                .map(|t| &t.object)
            else {
                return;
            };
            let has_object = fr.object.is_some();
            let object_equal = fr.object.as_ref() == Some(actual);
            if crate::resolver::factref_matches(has_object, object_equal) {
                current.remove(&(fr.subject.clone(), fr.predicate.clone()));
            }
        }
    }
}
