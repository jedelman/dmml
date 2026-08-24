//! `interpret::diverges` -- the primitive a drift/staleness check needs
//! (comparing two materialized snapshots, not just probing one). Exercised
//! directly against `Materialized::from_commits` rather than through any
//! machine/genesis content, since divergence is a property of two
//! `Materialized` values alone, nothing machine-specific.

use dmml::interpret::{diverges, Divergence, Materialized};
use dmml::lower::{LoweredCommit, Triple, TripleValue};

fn commit(triples: &[(&str, &str, TripleValue)]) -> LoweredCommit {
    LoweredCommit {
        predicate_verb: "mints".to_string(),
        consumes: vec![],
        produces: triples
            .iter()
            .map(|(s, p, v)| Triple {
                subject: s.to_string(),
                predicate: p.to_string(),
                object: v.clone(),
            })
            .collect(),
        via: None,
        responds_to: None,
    }
}

#[test]
fn identical_snapshots_diverge_on_nothing() {
    let before = Materialized::from_commits(&[commit(&[
        ("room/1", "opensTo", TripleValue::Node("room/2".to_string())),
        ("edge/12", "state", TripleValue::Node("locked".to_string())),
    ])]);
    let after = before.clone();
    assert_eq!(diverges(&before, &after), Vec::<Divergence>::new());
}

#[test]
fn a_changed_value_is_a_divergence() {
    let before = Materialized::from_commits(&[commit(&[(
        "edge/12",
        "state",
        TripleValue::Node("locked".to_string()),
    )])]);
    let after = Materialized::from_commits(&[commit(&[(
        "edge/12",
        "state",
        TripleValue::Node("unlocked".to_string()),
    )])]);
    assert_eq!(
        diverges(&before, &after),
        vec![Divergence {
            subject: "edge/12".to_string(),
            predicate: "state".to_string(),
            before: Some(TripleValue::Node("locked".to_string())),
            after: Some(TripleValue::Node("unlocked".to_string())),
        }],
        "a fact whose value changed between the two snapshots must be reported, \
         with both the old and new value carried, not just flagged as different"
    );
}

#[test]
fn a_fact_new_in_after_is_a_divergence_with_before_none() {
    let before = Materialized::from_commits(&[]);
    let after = Materialized::from_commits(&[commit(&[(
        "item/lantern",
        "state",
        TripleValue::Node("lit".to_string()),
    )])]);
    assert_eq!(
        diverges(&before, &after),
        vec![Divergence {
            subject: "item/lantern".to_string(),
            predicate: "state".to_string(),
            before: None,
            after: Some(TripleValue::Node("lit".to_string())),
        }],
        "something that came into existence only in `after` is a real divergence \
         (a creation), not something only a changed-value check would catch"
    );
}

#[test]
fn a_fact_only_in_before_is_a_divergence_with_after_none() {
    let before = Materialized::from_commits(&[commit(&[(
        "player",
        "holds",
        TripleValue::Node("key/7".to_string()),
    )])]);
    let after = Materialized::from_commits(&[]);
    assert_eq!(
        diverges(&before, &after),
        vec![Divergence {
            subject: "player".to_string(),
            predicate: "holds".to_string(),
            before: Some(TripleValue::Node("key/7".to_string())),
            after: None,
        }],
        "something retracted with nothing replacing it is a real divergence \
         (present in `before`, absent from `after`), not silently ignored"
    );
}

#[test]
fn multiple_divergences_come_back_sorted_by_subject_then_predicate() {
    let before = Materialized::from_commits(&[commit(&[
        ("z/node", "state", TripleValue::Node("a".to_string())),
        ("a/node", "z_predicate", TripleValue::Node("a".to_string())),
        ("a/node", "a_predicate", TripleValue::Node("a".to_string())),
    ])]);
    let after = Materialized::from_commits(&[commit(&[
        ("z/node", "state", TripleValue::Node("b".to_string())),
        ("a/node", "z_predicate", TripleValue::Node("b".to_string())),
        ("a/node", "a_predicate", TripleValue::Node("b".to_string())),
    ])]);
    let result = diverges(&before, &after);
    let order: Vec<(&str, &str)> = result
        .iter()
        .map(|d| (d.subject.as_str(), d.predicate.as_str()))
        .collect();
    assert_eq!(
        order,
        vec![
            ("a/node", "a_predicate"),
            ("a/node", "z_predicate"),
            ("z/node", "state"),
        ],
        "output order must be deterministic (subject, then predicate), not \
         HashMap/HashSet iteration order, so callers (and tests) can rely on it"
    );
}
