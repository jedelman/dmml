//! Property tests standing in for the resolver-fold frame property that
//! turned out not to be formally provable through Thermite's current
//! Map/Vec primitives (dev-journal/2026-08-17-map-fold-thermite-limits.md).
//! Not full provability -- proptest checks many random cases, not all of
//! them -- but real, automated, regression-catching assurance instead of
//! none.

use dmml::resolver::WorldState;
use proptest::prelude::*;

proptest! {
    #[test]
    fn assert_then_frame(a: u64, b: u64) {
        prop_assume!(a != b);

        let mut state = WorldState::new();

        state.assert_fact(b);
        assert!(state.is_current(b));

        state.assert_fact(a);
        assert!(state.is_current(a));
        assert!(state.is_current(b));
    }

    #[test]
    fn retract_then_frame(a: u64, b: u64) {
        prop_assume!(a != b);

        let mut state = WorldState::new();

        state.assert_fact(a);
        state.assert_fact(b);
        assert!(state.is_current(a));
        assert!(state.is_current(b));

        state.retract_fact(a);
        assert!(!state.is_current(a));
        assert!(state.is_current(b));
    }

    #[test]
    fn combined_commit_atomicity(retract_key: u64, assert_key: u64, other: u64) {
        prop_assume!(retract_key != assert_key);
        prop_assume!(retract_key != other);
        prop_assume!(assert_key != other);

        let mut state = WorldState::new();

        state.assert_fact(retract_key);
        state.assert_fact(other);
        assert!(state.is_current(retract_key));
        assert!(state.is_current(other));
        assert!(!state.is_current(assert_key));

        state.apply_combined_commit(retract_key, assert_key);
        assert!(!state.is_current(retract_key));
        assert!(state.is_current(assert_key));
        assert!(state.is_current(other));
    }

    #[test]
    fn retract_never_asserted_is_a_noop_elsewhere(a: u64, b: u64) {
        prop_assume!(a != b);

        let mut state = WorldState::new();

        state.assert_fact(b);
        assert!(!state.is_current(a));
        assert!(state.is_current(b));

        state.retract_fact(a);
        assert!(!state.is_current(a));
        assert!(state.is_current(b));
    }
}
