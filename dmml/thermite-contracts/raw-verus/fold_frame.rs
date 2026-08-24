//! Raw Verus, deliberately NOT a `.th` file: proves the resolver-fold
//! frame property that `dev-journal/2026-08-17-map-fold-thermite-limits.md`
//! found unreachable through Thermite's ordinary req/ens/fx surface (Verus's
//! automatic routing won't chain a flat `forall` through a recursive
//! predicate on its own) AND through Thermite's `lemma`/Lean-tactic
//! forge-tier escalation (`dev-journal/2026-08-17-raw-verus-induction.md`:
//! its Lean-exportable fragment structurally excludes if-expressions and
//! integer-literal match patterns in contract position, which any custom
//! recursive predicate is built from). Verified directly: `verus
//! fold_frame.rs` -> `4 verified, 0 errors`.
//!
//! Confirms the property is real and provable -- Thermite's DMML-author
//! surface just doesn't reach it today. This file is Dev-Lead/expert-Verus
//! work, not something dispatched to Kimi or expressed as a Thermite
//! contract; see the dev-journal entry for what that scope change means.

use vstd::prelude::*;

verus! {

/// Whether `k` appears among the first `i` elements of `v` (indices `0..i`).
/// Mirrors `dmml::thermite_contracts::vec_fold_needs_induction.th`'s
/// `spec_contains`, over `Seq<u64>` instead of the wrapped `Vec<u64>` --
/// isolating the actual induction from Thermite/vstd's Vec-wrapper plumbing.
pub open spec fn spec_contains(v: Seq<u64>, i: int, k: u64) -> bool
    decreases i
{
    if i <= 0 {
        false
    } else if v[i - 1] == k {
        true
    } else {
        spec_contains(v, i - 1, k)
    }
}

/// The frame lemma Thermite's automatic routing couldn't chain on its own:
/// if `v2` agrees with `v1` on every index below `v1.len()` (the property
/// `Vec::push`'s real postcondition actually gives a caller), then
/// `spec_contains` over any prefix length `i <= v1.len()` agrees too.
/// Proved by induction on `i` -- the actual inductive step Verus's
/// automatic routing wouldn't take without this explicit recursion.
pub proof fn lemma_contains_frame(v1: Seq<u64>, v2: Seq<u64>, i: int, k: u64)
    requires
        0 <= i <= v1.len(),
        forall|j: int| 0 <= j < v1.len() ==> v1[j] == v2[j],
    ensures
        spec_contains(v1, i, k) == spec_contains(v2, i, k),
    decreases i
{
    if i <= 0 {
        // Both sides false by definition -- base case, no recursion needed.
    } else {
        // Inductive hypothesis: the property holds one index earlier.
        lemma_contains_frame(v1, v2, i - 1, k);
        // v1[i-1] == v2[i-1] follows directly from the frame hypothesis,
        // since i - 1 < v1.len() here (i <= v1.len() and i > 0).
    }
}

/// The actual property `apply_assert`'s frame postcondition needed:
/// pushing `new_fact` onto `log` leaves every OTHER fact's `spec_contains`
/// status unchanged. `result` is `log` with `new_fact` appended at the end
/// (`result.len() == log.len() + 1`, `result[log.len()] == new_fact`,
/// `result` agrees with `log` on every earlier index) -- exactly
/// `Vec::push`'s real generated postcondition
/// (`thermite-lower/src/lower.rs:5135`), restated over `Seq` directly.
pub proof fn lemma_push_preserves_other_facts(log: Seq<u64>, result: Seq<u64>, new_fact: u64, other_fact: u64)
    requires
        new_fact != other_fact,
        result.len() == log.len() + 1,
        result[log.len() as int] == new_fact,
        forall|j: int| 0 <= j < log.len() ==> result[j] == log[j],
    ensures
        spec_contains(result, result.len() as int, other_fact) == spec_contains(log, log.len() as int, other_fact),
{
    // Unfold spec_contains(result, result.len(), other_fact) one step: it
    // checks index result.len()-1 == log.len(), which is the newly pushed
    // element (new_fact) -- not equal to other_fact by hypothesis, so it
    // falls through to spec_contains(result, log.len(), other_fact).
    // This equality is definitional (Verus unfolds the recursive spec fn
    // automatically here since result.len() and the guard are concrete
    // in the postcondition's own shape); the remaining gap is exactly
    // what lemma_contains_frame supplies.
    lemma_contains_frame(log, result, log.len() as int, other_fact);
}

fn main() {}

} // verus!
