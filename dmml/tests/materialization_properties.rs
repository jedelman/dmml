//! Frame-property test for the real retraction fold (issue #70's
//! suggested test plan): retracting one `(subject, predicate)` via a
//! `Fact` consume must leave every OTHER `(subject, predicate)` the
//! same target commit produced untouched -- the same frame property
//! `resolver_properties.rs` already checks for the abstract `WorldState`
//! model, checked here against the real `Materialized` fold instead.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::lower::{ConsumeRef, FactRef, LoweredCommit, StrongRef, Triple, TripleValue};
use proptest::prelude::*;

fn commit_with(triples: Vec<(String, String, String)>) -> LoweredCommit {
    LoweredCommit {
        predicate_verb: "becomes".to_string(),
        consumes: vec![],
        produces: triples
            .into_iter()
            .map(|(subject, predicate, object)| Triple {
                subject,
                predicate,
                object: TripleValue::Node(object),
            })
            .collect(),
        refs: std::collections::HashMap::new(),
    }
}

proptest! {
    #[test]
    fn fact_consume_retract_then_frame(
        subject_a in "[a-z]{1,8}",
        predicate_a in "[a-z]{1,8}",
        subject_b in "[a-z]{1,8}",
        predicate_b in "[a-z]{1,8}",
    ) {
        prop_assume!((subject_a.clone(), predicate_a.clone()) != (subject_b.clone(), predicate_b.clone()));

        let mint = IdentifiedCommit {
            uri: "at://did:plc:aaaa/collection/mint".to_string(),
            cid: "bafymint".to_string(),
            commit: commit_with(vec![
                (subject_a.clone(), predicate_a.clone(), "val_a".to_string()),
                (subject_b.clone(), predicate_b.clone(), "val_b".to_string()),
            ]),
        };

        let mut consumer_commit = commit_with(vec![]);
        consumer_commit.consumes.push(ConsumeRef::Fact(FactRef {
            commit: StrongRef {
                uri: "at://did:plc:aaaa/collection/mint".to_string(),
                cid: "bafymint".to_string(),
            },
            subject: subject_a.clone(),
            predicate: predicate_a.clone(),
            object: None,
        }));
        let consumer = IdentifiedCommit {
            uri: "at://did:plc:aaaa/collection/consumer".to_string(),
            cid: "bafyconsumer".to_string(),
            commit: consumer_commit,
        };

        let world = Materialized::from_identified_commits(&[mint, consumer]);

        prop_assert_eq!(world.current_value(&subject_a, &predicate_a), None);
        prop_assert_eq!(
            world.current_value(&subject_b, &predicate_b),
            Some(&TripleValue::Node("val_b".to_string()))
        );
    }
}
