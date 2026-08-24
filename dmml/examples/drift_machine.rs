//! A worked test of "try DMML first," applied to the drift-machine Jason
//! blessed 2026-08-20 right after `interpret::diverges` was designed and
//! built for exactly this. Two real DMML commits stand in for "what a
//! sense-machine quoted into memory" and "what's true now"; `diverges`
//! (the one new, genuinely interpreter-side primitive -- comparing across
//! two `Materialized` snapshots isn't expressible as a guard against a
//! single one, see `SPEC.md` §19.2) finds what changed; a third, ordinary
//! DMML `commit` block -- no bespoke Rust struct, no new vocabulary
//! mechanism -- represents the drift itself as content. Confirms the
//! actual claim: only the *comparison* needed a new primitive, not the
//! drift-machine's own shape. Run with `cargo run -p dmml --example
//! drift_machine`.

use dmml::interpret::{diverges, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

const MEMORY_SRC: &str = r#"
commit remembered {
  declare attribute state

  window state open
}
"#;

const CURRENT_SRC: &str = r#"
commit ground_truth {
  declare attribute state

  window state closed
}
"#;

fn commit_of(doc: &dmml::Document) -> &dmml::ast::CommitStmt {
    doc.items
        .iter()
        .find_map(|item| match item {
            TopLevelItem::Commit(c) => Some(c),
            _ => None,
        })
        .expect("the document has a commit")
}

fn materialize(src: &str) -> Materialized {
    let doc = dmml::parse(src).expect("should parse");
    let commit = commit_of(&doc);
    validate_declarations(commit).expect("every predicate should be self-declared");
    let lowered = lower::lower_commit(commit);
    Materialized::from_commits(&[lowered])
}

fn main() {
    println!("=== memory (what a sense-machine quoted earlier) ===\n{MEMORY_SRC}");
    let memory_world = materialize(MEMORY_SRC);

    println!("=== current (what's true now) ===\n{CURRENT_SRC}");
    let current_world = materialize(CURRENT_SRC);

    let found = diverges(&memory_world, &current_world);
    println!("diverges(memory, current) = {found:?}\n");
    assert_eq!(found.len(), 1, "exactly one fact drifted: window's state");
    let d = &found[0];
    assert_eq!(d.subject, "window");
    assert_eq!(d.predicate, "state");

    // The comparison is Rust (interpret::diverges); representing what it
    // found is ordinary DMML content -- no bespoke Drift struct, no new
    // serialization path. Built here by formatting the divergence's own
    // fields into DMML source text, same as any other authored commit.
    let drift_src = format!(
        r#"
commit notices_drift {{
  declare relation driftedSubject
  declare attribute driftPredicate
  declare attribute driftOldValue
  declare attribute driftNewValue
  declare relation noticedChange

  drift/1 driftedSubject {subject}
  drift/1 driftPredicate "{predicate}"
  drift/1 driftOldValue {old}
  drift/1 driftNewValue {new}
  player noticedChange drift/1
}}
"#,
        subject = d.subject,
        predicate = d.predicate,
        old = match d.before.as_ref().expect("this divergence has a before-value") {
            dmml::lower::TripleValue::Node(v) => v.clone(),
            other => panic!("unexpected before-value shape: {other:?}"),
        },
        new = match d.after.as_ref().expect("this divergence has an after-value") {
            dmml::lower::TripleValue::Node(v) => v.clone(),
            other => panic!("unexpected after-value shape: {other:?}"),
        },
    );

    println!("=== the drift itself, authored as ordinary DMML content ===\n{drift_src}");
    let drift_world = materialize(&drift_src);
    println!(
        "materialized drift record: {} (subject, predicate) pairs\n",
        drift_world.len()
    );
    assert_eq!(
        drift_world.current_value("drift/1", "driftOldValue"),
        Some(&dmml::lower::TripleValue::Node("open".to_string()))
    );
    assert_eq!(
        drift_world.current_value("drift/1", "driftNewValue"),
        Some(&dmml::lower::TripleValue::Node("closed".to_string()))
    );
    assert_eq!(
        drift_world.current_value("player", "noticedChange"),
        Some(&dmml::lower::TripleValue::Node("drift/1".to_string()))
    );

    println!(
        "=== done: diverges() is the only new Rust; the drift-machine's own \
         shape is pure DMML content ==="
    );
}
