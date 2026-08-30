//! Property test for `Materialized::from_commits`: checks last-write-wins
//! folding against an independently computed reference, over randomly
//! generated commit logs with a small subject/predicate alphabet (forcing
//! real overlap across many commits) -- the same methodology
//! `validate_properties.rs` used for the combinatoric validation pass,
//! applied here to the actual resolve fold.

use dmml::interpret::Materialized;
use dmml::lower::{LoweredCommit, Triple, TripleValue};
use proptest::prelude::*;
use std::collections::HashMap;

fn gen_triple() -> impl Strategy<Value = Triple> {
    let subject = prop_oneof![Just("s0"), Just("s1"), Just("s2")];
    let predicate = prop_oneof![Just("p0"), Just("p1")];
    let value = prop::num::u8::ANY;
    (subject, predicate, value).prop_map(|(s, p, v)| Triple {
        subject: s.to_string(),
        predicate: p.to_string(),
        object: TripleValue::Number(v.to_string()),
    })
}

fn gen_commit() -> impl Strategy<Value = LoweredCommit> {
    prop::collection::vec(gen_triple(), 0..5).prop_map(|produces| LoweredCommit {
        predicate_verb: "mints".to_string(),
        consumes: vec![],
        produces,
        refs: std::collections::HashMap::new(),
    })
}

proptest! {
    #[test]
    fn matches_independent_last_write_wins_reference(
        commits in prop::collection::vec(gen_commit(), 0..10)
    ) {
        // Independent reference: walk the same commits/triples directly,
        // last write per (subject, predicate) wins.
        let mut expected: HashMap<(String, String), TripleValue> = HashMap::new();
        for commit in &commits {
            for triple in &commit.produces {
                expected.insert(
                    (triple.subject.clone(), triple.predicate.clone()),
                    triple.object.clone(),
                );
            }
        }

        let m = Materialized::from_commits(&commits);

        prop_assert_eq!(m.len(), expected.len());
        for ((subject, predicate), value) in &expected {
            prop_assert_eq!(m.current_value(subject, predicate), Some(value));
        }
    }
}
