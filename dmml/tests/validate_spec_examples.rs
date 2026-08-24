//! Independent verification of `dmml::validate::validate_declarations`
//! against VALIDATION_SPEC.md's five worked examples, plus the two cases
//! the spec states only as rules (4/5) and never works a full example
//! for -- a fact nested inside `produces { }`, and a `consumes`-only
//! commit referencing an undeclared predicate. This is the genuinely
//! combinatorial follow-up to lower_spec_examples.rs: a two-pass,
//! set-based check over unbounded input, not a flat walk over a closed
//! set of variants.

use dmml::ast::TopLevelItem;
use dmml::validate::{validate_declarations, UndeclaredPredicate};

fn validate_first_commit(src: &str) -> Result<(), Vec<UndeclaredPredicate>> {
    let doc = dmml::parse(src).expect("should parse");
    let TopLevelItem::Commit(commit) = &doc.items[0] else {
        panic!("expected a commit");
    };
    validate_declarations(commit)
}

#[test]
fn example_1_declared_before_use_is_ok() {
    let src = r#"
commit mints {
  declare relation opensTo
  room/42 opensTo room/43
}
"#;
    assert_eq!(validate_first_commit(src), Ok(()));
}

#[test]
fn example_2_declared_after_use_is_ok_order_independent() {
    let src = r#"
commit mints {
  room/42 opensTo room/43
  declare relation opensTo
}
"#;
    assert_eq!(validate_first_commit(src), Ok(()));
}

#[test]
fn example_3_never_declared_is_an_error() {
    let src = r#"
commit mints {
  room/42 opensTo room/43
}
"#;
    let result = validate_first_commit(src);
    let errs = result.expect_err("should be undeclared");
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].predicate, "opensTo");
}

#[test]
fn example_4_rdf_type_never_needs_declaring() {
    let src = r#"
commit mints {
  room/42 a Room
}
"#;
    assert_eq!(validate_first_commit(src), Ok(()));
}

#[test]
fn example_5_multiple_undeclared_reported_in_order() {
    let src = r#"
commit mints {
  room/42 opensTo room/43
  room/42 dampness 0.4
}
"#;
    let result = validate_first_commit(src);
    let errs = result.expect_err("should be undeclared");
    assert_eq!(errs.len(), 2);
    assert_eq!(errs[0].predicate, "opensTo");
    assert_eq!(errs[1].predicate, "dampness");
}

/// Not a worked example in the spec -- only stated as rule 4 ("a fact
/// inside an explicit produces block obeys the identical rule"). Tests
/// whether the implementation actually generalizes the rule to a nested
/// fact, not just the bare-fact case every worked example used.
#[test]
fn undeclared_predicate_inside_explicit_produces_block_is_still_an_error() {
    let src = r#"
commit mints {
  declare relation opensTo
  produces {
    room/42 opensTo room/43
    room/42 dampness 0.4
  }
}
"#;
    let result = validate_first_commit(src);
    let errs = result.expect_err("should be undeclared");
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].predicate, "dampness");
}

/// Not a worked example -- only stated as rule 5 ("Consumes items are
/// skipped entirely"). A consumes-only commit with an undeclared-looking
/// predicate reference must NOT error.
#[test]
fn consumes_only_commit_is_never_checked() {
    let src = r#"
commit becomes {
  consumes {
    fact at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789
      (cid: bafyabcxyz) {
      subject: room/42
      predicate: locked
    }
  }
}
"#;
    assert_eq!(validate_first_commit(src), Ok(()));
}
