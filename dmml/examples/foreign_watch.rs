//! Proves SPEC.md §19.3's actual claim: cross-repo watching needs zero
//! new primitives, only a discipline about which `Materialized` feeds
//! which function. Two independent commit sets stand in for "our own
//! repo" (holding a `reach` reference, plus our own prior quote of the
//! foreign fact) and "a foreign repo" (their own current truth) --
//! materialized separately, NEVER folded together, exactly the rule
//! this file exists to demonstrate. `diverges` runs between our quote
//! and their fresh state -- the identical call §19.2's same-repo memory
//! comparison already uses, unaware either side is foreign. Run with
//! `cargo run -p dmml --example foreign_watch`.

use dmml::interpret::{diverges, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

/// Our own repo's `reach` reference (SPEC.md §6): a fact about *what
/// we're watching*, legitimately guardable since it's ours. Kept as its
/// own commit, deliberately -- see `QUOTE_SRC` below for why it must
/// NOT be part of what gets diffed against the foreign side.
const REACH_SRC: &str = r#"
commit watches {
  declare attribute foreignUri
  declare attribute foreignCid

  world/seed foreignUri "at://did:plc:foreign/org.jason-edelman.writtenworld.commit/rkey123"
  world/seed foreignCid "bafyForeignHead"
}
"#;

/// Our own quote of what we last observed about the foreign window --
/// minted the same way any §19.2 memory commit is, using the SAME
/// subject the foreign repo itself uses ("window"), not a
/// locally-renamed alias, so `diverges` can match it against the
/// foreign side's own naming.
///
/// **Kept separate from `REACH_SRC` on purpose -- a real finding from
/// actually running this example, not designed in up front.** A first
/// draft materialized the *whole* local repo (reach reference and quote
/// together) and diffed that against the foreign side; `diverges`
/// correctly reported five divergences, not one -- it also found the
/// `reach` reference itself and the `declare`d predicates' own
/// `rdf:type` triples, since none of those exist on the foreign side
/// either. All true, none of it useful: a drift check needs to compare
/// only the subset of our own repo that actually quotes foreign
/// content, not everything else we happen to know locally. The fix
/// isn't a `diverges` change -- it never claimed to compare anything
/// but the two `Materialized` values it's given -- it's scoping which
/// commit gets materialized for the comparison.
const QUOTE_SRC: &str = r#"
commit remembered {
  declare attribute state

  window state open
}
"#;

/// A separate repo we do not own and never consume from -- only its
/// own commits determine its own materialized truth, per repo-local
/// determinism (#69). This is fetched fresh, never folded into our own
/// graph above.
const FOREIGN_SRC: &str = r#"
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
    println!("=== our own reach reference (a purely local fact) ===\n{REACH_SRC}");
    let reach_world = materialize(REACH_SRC);

    println!("=== our own prior quote of the foreign window ===\n{QUOTE_SRC}");
    let quote_world = materialize(QUOTE_SRC);

    println!("=== the foreign repo, fetched fresh, materialized SEPARATELY ===\n{FOREIGN_SRC}");
    let foreign_world = materialize(FOREIGN_SRC);

    // The one discipline SPEC.md §19.3 states as a hard rule:
    // foreign_world is never folded into anything we own -- not the
    // reach reference, not the quote. Each stays its own, independent
    // Materialized; foreign_world never feeds a guard or
    // may_fire/commit_fires_transition call over our own machines.
    assert_ne!(
        reach_world.current_value("world/seed", "foreignUri"),
        None,
        "the reach reference is a local fact -- legitimately ours to check"
    );

    // Comparing *only* the quote against the foreign side -- not our
    // whole repo, per the finding above.
    let found = diverges(&quote_world, &foreign_world);
    println!("diverges(our quote alone, their fresh state) = {found:?}\n");
    assert_eq!(found.len(), 1, "exactly one fact drifted: window's state");
    let d = &found[0];
    assert_eq!(d.subject, "window");
    assert_eq!(d.predicate, "state");
    assert_eq!(d.before, Some(dmml::lower::TripleValue::Node("open".to_string())));
    assert_eq!(d.after, Some(dmml::lower::TripleValue::Node("closed".to_string())));

    println!(
        "=== done: the foreign repo was read and compared, never consumed. \
         Writing an updated quote back into our own repo (SPEC.md §19.2's \
         memory-commit mechanism, already proven in drift_machine.rs) is \
         ordinary same-repo mechanics from here -- the cross-repo boundary \
         was only ever crossed at fetch time. ==="
    );
}
