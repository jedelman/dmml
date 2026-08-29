//! End-to-end: real JSON commit authoring -> AST -> lower -> materialize,
//! checking the actual `current_value` a resolver would answer.

use dmml::from_json::commit_from_json;
use dmml::interpret::Materialized;
use dmml::lower::{lower_commit, LoweredCommit, TripleValue};

fn lower_all_commits(jsons: &[&str]) -> Vec<LoweredCommit> {
    jsons
        .iter()
        .map(|json| {
            let commit = commit_from_json(json).expect("should build");
            lower_commit(&commit)
        })
        .collect()
}

#[test]
fn single_commit_current_values() {
    let commits = lower_all_commits(&[r#"{
        "verb": "mints",
        "declares": [{"kind": "relation", "name": "opensTo"}],
        "facts": [
            {"subject": "room/42", "predicate": "a", "object": {"kind": "node", "value": "Room"}},
            {"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}}
        ]
    }"#]);
    let m = Materialized::from_commits(&commits);
    assert_eq!(
        m.current_value("room/42", "rdf:type"),
        Some(&TripleValue::Node("Room".to_string()))
    );
    assert_eq!(
        m.current_value("room/42", "opensTo"),
        Some(&TripleValue::Node("room/43".to_string()))
    );
    assert_eq!(m.current_value("room/42", "nonexistent"), None);
    // 3 distinct (subject, predicate) pairs: the `declare` itself lowers
    // to a triple too (("opensTo", "rdf:type") -> Node("Relation")), plus
    // the two facts on room/42.
    assert_eq!(
        m.current_value("opensTo", "rdf:type"),
        Some(&TripleValue::Node("Relation".to_string()))
    );
    assert_eq!(m.len(), 3);
}

#[test]
fn later_commit_overwrites_earlier_for_same_subject_predicate() {
    let commits = lower_all_commits(&[
        r#"{"verb": "mints",
            "facts": [{"subject": "room/42", "predicate": "locked", "object": {"kind": "boolean", "value": true}}]}"#,
        r#"{"verb": "unlocks",
            "facts": [{"subject": "room/42", "predicate": "locked", "object": {"kind": "boolean", "value": false}}]}"#,
    ]);
    let m = Materialized::from_commits(&commits);
    assert_eq!(
        m.current_value("room/42", "locked"),
        Some(&TripleValue::Boolean(false))
    );
    assert_eq!(m.len(), 1);
}

#[test]
fn unrelated_subject_predicate_pairs_are_untouched_by_a_later_commit() {
    let commits = lower_all_commits(&[
        r#"{"verb": "mints",
            "facts": [
                {"subject": "room/42", "predicate": "locked", "object": {"kind": "boolean", "value": true}},
                {"subject": "room/43", "predicate": "locked", "object": {"kind": "boolean", "value": true}}
            ]}"#,
        r#"{"verb": "unlocks",
            "facts": [{"subject": "room/42", "predicate": "locked", "object": {"kind": "boolean", "value": false}}]}"#,
    ]);
    let m = Materialized::from_commits(&commits);
    assert_eq!(
        m.current_value("room/42", "locked"),
        Some(&TripleValue::Boolean(false))
    );
    // room/43's value is untouched by the second commit, which never
    // mentions it -- the frame property this session spent so long
    // proving for the abstract fold model, now checked for the real
    // Triple-based one.
    assert_eq!(
        m.current_value("room/43", "locked"),
        Some(&TripleValue::Boolean(true))
    );
}

#[test]
fn empty_log_has_no_current_values() {
    let m = Materialized::from_commits(&[]);
    assert!(m.is_empty());
    assert_eq!(m.current_value("anything", "anything"), None);
}
