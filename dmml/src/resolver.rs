//! Reference resolver state model for DMML: SPEC.md SS5's fold ("unioning
//! the asserted content of relevant, valid commits ... a commit with no
//! assertions is a pure retraction -- subtracts, never adds") plus the
//! validation-gate invariants this session pinned as Thermite contracts
//! in `thermite-contracts/*.th`.
//!
//! Two different kinds of assurance below, and this module is honest
//! about which is which:
//!
//! - Four of five standalone gate functions (`resolves`, `factref_matches`,
//!   `cross_repo_commit_valid`, `commit_valid_despite_dangling_factref`)
//!   are direct, traceable transcriptions of Thermite contracts that
//!   certified L3 (a real Verus proof) -- see each function's doc comment
//!   for which `.th` file. The fifth, `commit_is_valid`, is NOT currently
//!   one of them -- see its own doc comment for why (a deliberate,
//!   on-the-record break of the formal-verification boundary, pending a
//!   full cryptographic re-derivation of this resolver).
//! - `WorldState`'s fold operations (`assert_fact`/`retract_fact`/
//!   `apply_combined_commit`) implement the same invariants the
//!   atomicity contract (`retract_assert_atomicity.th`) proved for a
//!   single combined commit, extended to a real, growable, multi-commit
//!   log -- exactly the operation `dev-journal/2026-08-17-map-fold-
//!   thermite-limits.md` found isn't provable through Thermite's current
//!   `Map`/`Vec` primitives (the frame property: does asserting one fact
//!   leave every other fact's resolved status alone). Covered here by
//!   property tests (`resolver_properties.rs`) instead of a formal proof
//!   -- not full provability, but real, checked behavior, per the
//!   decision to prioritize working code over chasing an induction proof
//!   that would have been a materially bigger, riskier lift. As of the
//!   Datalog cutover, also cross-checked by `datalog_worldstate`'s crepe
//!   fixpoint oracle -- a one-shot batch recomputation of `is_current`
//!   over a whole recorded operation log, not a live replacement for
//!   this struct (crepe's `run(self)` consumes its instance; there's no
//!   incremental "keep the derived state, add one more fact" shape for
//!   an always-growing, live-mutated struct like this one to take).

use std::collections::HashMap;

/// A repo's own commit log, modeled as two disjoint fact-id presence
/// sets. Append-only, matching SPEC.md SS3/SS7: retraction is
/// bookkeeping (a second, additive record), never physical deletion of a
/// prior commit's contribution -- so `asserted` never shrinks, and
/// `retract_fact` records a retraction rather than removing anything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldState {
    asserted: HashMap<u64, ()>,
    retracted: HashMap<u64, ()>,
}

impl WorldState {
    pub fn new() -> Self {
        WorldState::default()
    }

    /// Whether `fact` is part of the currently-resolved view: asserted,
    /// and not since retracted.
    pub fn is_current(&self, fact: u64) -> bool {
        self.asserted.contains_key(&fact) && !self.retracted.contains_key(&fact)
    }

    /// Records a new assertion. Idempotent: asserting an already-asserted
    /// fact again is a no-op on `is_current` (still true), matching
    /// append-only semantics -- this is a presence record, not a counter.
    pub fn assert_fact(&mut self, fact: u64) {
        self.asserted.insert(fact, ());
    }

    /// Records a retraction. Per `fact_retraction_fails_open.th`'s own
    /// framing ("an ordinary dangling reference, not a structural
    /// violation"), retracting a fact that was never asserted is not an
    /// error -- it simply leaves `is_current` false, same as it already
    /// was.
    pub fn retract_fact(&mut self, fact: u64) {
        self.retracted.insert(fact, ());
    }

    /// The atomic combined-commit operation `retract_assert_atomicity.th`
    /// proved for a single call: retract one fact and assert another in
    /// one step. Both halves always happen together -- there is no
    /// return path that performs only one, the same property the proved
    /// contract pinned.
    pub fn apply_combined_commit(&mut self, retract_key: u64, assert_key: u64) {
        self.retract_fact(retract_key);
        self.assert_fact(assert_key);
    }
}

/// Repo-local determinism (`repo_local_determinism.th`, L3-certified):
/// resolution depends only on `commit_in_own_log`, never on
/// `foreign_repo_accepted` -- the exact invariant behind the historical
/// `respondsTo` bug (SPEC.md SS18, issue #69). The parameter is kept
/// (not dropped from the signature) so a caller can't accidentally lose
/// track of which inputs are in play; the leading underscore is the only
/// enforcement left once the contract itself is gone -- Rust's own
/// unused-parameter lint plus the doc comment stand in for the proof.
pub fn resolves(commit_in_own_log: bool, _foreign_repo_accepted: bool) -> bool {
    commit_in_own_log
}

/// **FORMAL-VERIFICATION BOUNDARY BROKEN HERE, DELIBERATELY, 2026-08-29.**
/// This function used to be a direct, unmodified transcription of
/// `field_inertness_independence.th` (L3-certified, Verus-proven):
/// `commit_is_valid(consumes_are_valid, _via_present, _responds_to_present)
/// -> consumes_are_valid`, proving validity depends on neither `via` nor
/// `respondsTo` being present. That proof is now stale, not just
/// superseded in spirit: `via`/`respondsTo` stopped being their own
/// dedicated fields at all (collapsed into `ast::CommitStmt.refs`'s open
/// role map alongside the new `requires` role -- see that field's own doc
/// comment), and this function has been hand-edited, outside Thermite/
/// Verus, to add a real, non-inert dependency on `requires_are_valid` --
/// exactly the kind of change every other doc comment in this file warns
/// against making casually. Done anyway, explicitly, on the record: a
/// full cryptographic re-derivation of this resolver is planned this
/// week, at which point this function's contract needs re-proving from
/// scratch regardless, so preserving the old proof's pristine text here
/// in the meantime bought nothing but a false sense of assurance. Until
/// that re-derivation lands, treat this function as ordinary
/// (well-tested, `resolver_gates.rs` covers it) but NOT formally verified
/// Rust, unlike its four siblings below, which are untouched and still
/// are.
///
/// `requires_are_valid` should be `true` only if every `StrongRef` under
/// the commit's `refs["requires"]` role actually resolves -- this
/// function itself never does that resolution (no history/store access
/// at this layer, same reason `consumes_are_valid` is a precomputed bool
/// here rather than a lookup); a caller with real access to a commit
/// history computes it and passes the result in, same calling convention
/// every other gate in this module already uses.
pub fn commit_is_valid(consumes_are_valid: bool, requires_are_valid: bool) -> bool {
    consumes_are_valid && requires_are_valid
}

/// FactRef wildcard matching (`factref_wildcard_matching.th`,
/// L3-certified): an omitted object matches every candidate for the same
/// (subject, predicate); a specified object requires exact equality.
pub fn factref_matches(has_object: bool, object_equal: bool) -> bool {
    !has_object || object_equal
}

/// Cross-repo consume fails closed on the whole commit
/// (`cross_repo_consume_fails_closed.th`, L3-certified): a commit whose
/// `consumes` claim illegally crosses a repository boundary is invalid
/// outright, regardless of whether the rest of the commit would
/// otherwise have been fine.
pub fn cross_repo_commit_valid(is_cross_repo_consume: bool, otherwise_valid: bool) -> bool {
    !is_cross_repo_consume && otherwise_valid
}

/// Fact-level retraction fails open on the carrying commit
/// (`fact_retraction_fails_open.th`, L3-certified): a dangling fact-level
/// reference never invalidates the commit that carries it -- the
/// opposite posture from `cross_repo_commit_valid`, deliberately, for a
/// different severity of problem (SPEC.md SS7).
pub fn commit_valid_despite_dangling_factref(
    _factref_dangles: bool,
    rest_of_commit_valid: bool,
) -> bool {
    rest_of_commit_valid
}
