//! A Datalog *equivalence oracle* for `resolver::WorldState`'s fold --
//! NOT a replacement. `resolver.rs`'s own doc comment already names why:
//! the frame property (does asserting one fact leave every other fact's
//! resolved status alone) is exactly what `dev-journal/2026-08-17-map-
//! fold-thermite-limits.md` found isn't provable through Thermite's
//! current `Map`/`Vec` primitives, and it's currently covered only by
//! `resolver_properties.rs`'s proptest cases -- real, but sampled, not
//! exhaustive.
//!
//! `WorldState` itself stays exactly as it is: a live, growable,
//! incrementally-mutated struct a caller keeps across many separate
//! `assert_fact`/`retract_fact`/`apply_combined_commit` calls over time.
//! Crepe's `Crepe::run(self)` consumes its instance by value and returns
//! a finished fixpoint -- there is no incremental "add one more fact and
//! keep the old derived state" API, so a live `WorldState` replacement is
//! not a shape crepe can take at all, only a one-shot batch computation
//! over a whole recorded operation log. That's exactly what this module
//! is: given the *same* sequence of operations a real `WorldState` was
//! driven with, recompute `is_current` for every fact from scratch via a
//! real fixpoint, and let a test assert the two computations agree.
//!
//! The rule itself is deliberately simple, because `WorldState::is_current`
//! itself is order-independent within a single operation log -- it never
//! asks *when* a fact was asserted or retracted relative to other
//! operations on other facts, only whether an assert and a later-or-
//! earlier retract exist anywhere in the log for that fact:
//! `Current(f) <- Asserted(f), !Retracted(f)`. One stratum: `Asserted`
//! and `Retracted` are both pure `@input`, and neither is ever derived
//! from `Current`, so there's no cycle for the negation to fall inside.

use std::collections::HashSet;

use crate::resolver::WorldState;

/// One operation from a `WorldState` caller's real history, in the exact
/// order they were applied -- mirrors the three real mutating operations
/// `resolver::WorldState` exposes. `Combined` is kept as its own variant
/// (rather than expanded into a bare `Retract`+`Assert` pair by the
/// caller) so a test driving both the oracle and a real `WorldState` from
/// the same op list can call `WorldState::apply_combined_commit` for it,
/// exercising the same atomic entry point `retract_assert_atomicity.th`
/// certified, not two separate calls that happen to have the same effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Assert(u64),
    Retract(u64),
    Combined { retract_key: u64, assert_key: u64 },
}

crepe::crepe! {
    @input
    struct Asserted(u64);
    @input
    struct Retracted(u64);

    @output
    struct Current(u64);

    Current(f) <- Asserted(f), !Retracted(f);
}

/// Replays `ops` through a real `WorldState` and, independently, through
/// the crepe fixpoint above, then returns both computations' verdicts for
/// `query` as `(real, oracle)` -- a caller (a test, here) decides what to
/// do if they disagree, rather than this function panicking or silently
/// picking one.
pub fn agrees_with_world_state(ops: &[Op], query: u64) -> (bool, bool) {
    let mut real = WorldState::new();
    let mut runtime = Crepe::new();

    for op in ops {
        match *op {
            Op::Assert(f) => {
                real.assert_fact(f);
                runtime.extend([Asserted(f)]);
            }
            Op::Retract(f) => {
                real.retract_fact(f);
                runtime.extend([Retracted(f)]);
            }
            Op::Combined { retract_key, assert_key } => {
                real.apply_combined_commit(retract_key, assert_key);
                runtime.extend([Retracted(retract_key)]);
                runtime.extend([Asserted(assert_key)]);
            }
        }
    }

    let (current,) = runtime.run();
    let current_facts: HashSet<u64> = current.into_iter().map(|Current(f)| f).collect();

    (real.is_current(query), current_facts.contains(&query))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn assert_agrees(ops: &[Op], query: u64) {
        let (real, oracle) = agrees_with_world_state(ops, query);
        assert_eq!(
            real, oracle,
            "WorldState and the Datalog oracle disagree on is_current({query}) for ops {ops:?}"
        );
    }

    #[test]
    fn assert_then_frame() {
        assert_agrees(&[Op::Assert(1), Op::Assert(2)], 1);
        assert_agrees(&[Op::Assert(1), Op::Assert(2)], 2);
    }

    #[test]
    fn retract_then_frame() {
        let ops = [Op::Assert(1), Op::Assert(2), Op::Retract(1)];
        assert_agrees(&ops, 1);
        assert_agrees(&ops, 2);
    }

    #[test]
    fn combined_commit_atomicity() {
        let ops = [
            Op::Assert(1),
            Op::Assert(3),
            Op::Combined { retract_key: 1, assert_key: 2 },
        ];
        assert_agrees(&ops, 1);
        assert_agrees(&ops, 2);
        assert_agrees(&ops, 3);
    }

    #[test]
    fn retract_never_asserted_is_a_noop_elsewhere() {
        let ops = [Op::Assert(2), Op::Retract(1)];
        assert_agrees(&ops, 1);
        assert_agrees(&ops, 2);
    }

    #[test]
    fn double_assert_is_idempotent() {
        assert_agrees(&[Op::Assert(1), Op::Assert(1)], 1);
    }

    #[test]
    fn retract_after_reassert_still_wins() {
        // WorldState::is_current never consults order between operations
        // on the same fact -- a retraction anywhere in the log beats an
        // assertion anywhere in the log, regardless of which came last.
        // This is the one property most worth pinning explicitly: it's
        // the part of the fold's semantics most likely to be *assumed*
        // to be order-sensitive (a naive reader might expect "assert
        // after retract" to resurrect the fact) when it deliberately
        // isn't -- see resolver.rs's own append-only framing.
        assert_agrees(&[Op::Retract(1), Op::Assert(1)], 1);
    }

    proptest! {
        #[test]
        fn oracle_agrees_on_arbitrary_op_sequences(
            ops in proptest::collection::vec(
                prop_oneof![
                    (0u64..8).prop_map(Op::Assert),
                    (0u64..8).prop_map(Op::Retract),
                    (0u64..8, 0u64..8).prop_map(|(retract_key, assert_key)| Op::Combined {
                        retract_key,
                        assert_key,
                    }),
                ],
                0..20,
            ),
            query in 0u64..8,
        ) {
            assert_agrees(&ops, query);
        }
    }
}
