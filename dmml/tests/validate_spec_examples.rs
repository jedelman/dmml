//! Independent verification of `dmml::validate::validate_declarations`
//! against VALIDATION_SPEC.md's five worked examples, plus the two cases
//! the spec states only as rules (4/5) and never works a full example
//! for -- a fact nested inside `produces { }`, and a `consumes`-only
//! commit referencing an undeclared predicate. This is the genuinely
//! combinatorial follow-up to lower_spec_examples.rs: a two-pass,
//! set-based check over unbounded input, not a flat walk over a closed
//! set of variants.
//!
//! Every case an agent could actually author goes through
//! `from_json::commit_from_json`, same as real content. The one exception
//! (`undeclared_predicate_inside_explicit_produces_block_is_still_an_error`)
//! builds `ast::CommitStmt` by hand: JSON authoring never produces a
//! `CommitItem::Produces` (see `from_json`'s own doc comment -- bare facts
//! are the sole JSON shape, since the two forms are semantically
//! identical), so this checks `validate_declarations` still generalizes
//! to that AST shape directly, the same way `validate_properties.rs`
//! already builds fixtures straight against the AST rather than through
//! JSON.

use dmml::ast::{self, CommitItem, DeclKind, DeclareStmt, FactStmt, Literal, PredicateRef, Span, Value};
use dmml::from_json::commit_from_json;
use dmml::validate::{validate_declarations, UndeclaredPredicate};

fn validate_json(json: &str) -> Result<(), Vec<UndeclaredPredicate>> {
    let commit = commit_from_json(json).expect("should build");
    validate_declarations(&commit)
}

#[test]
fn example_1_declared_before_use_is_ok() {
    let json = r#"{
        "verb": "mints",
        "declares": [{"kind": "relation", "name": "opensTo"}],
        "facts": [{"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}}]
    }"#;
    assert_eq!(validate_json(json), Ok(()));
}

#[test]
fn example_2_declared_after_use_is_ok_order_independent() {
    // Declares and facts live in separate arrays in the JSON shape, so
    // there is no literal "declared after use" to author -- this checks
    // the same order-independence the spec names, just expressed as
    // declares/facts arriving in whichever array order.
    let json = r#"{
        "verb": "mints",
        "facts": [{"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}}],
        "declares": [{"kind": "relation", "name": "opensTo"}]
    }"#;
    assert_eq!(validate_json(json), Ok(()));
}

#[test]
fn example_3_never_declared_is_an_error() {
    let json = r#"{
        "verb": "mints",
        "facts": [{"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}}]
    }"#;
    let errs = validate_json(json).expect_err("should be undeclared");
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].predicate, "opensTo");
}

#[test]
fn example_4_rdf_type_never_needs_declaring() {
    let json = r#"{
        "verb": "mints",
        "facts": [{"subject": "room/42", "predicate": "a", "object": {"kind": "node", "value": "Room"}}]
    }"#;
    assert_eq!(validate_json(json), Ok(()));
}

#[test]
fn example_5_multiple_undeclared_reported_in_order() {
    let json = r#"{
        "verb": "mints",
        "facts": [
            {"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}},
            {"subject": "room/42", "predicate": "dampness", "object": {"kind": "number", "value": "0.4"}}
        ]
    }"#;
    let errs = validate_json(json).expect_err("should be undeclared");
    assert_eq!(errs.len(), 2);
    assert_eq!(errs[0].predicate, "opensTo");
    assert_eq!(errs[1].predicate, "dampness");
}

/// Not a worked example in the spec -- only stated as rule 4 ("a fact
/// inside an explicit produces block obeys the identical rule"). Tests
/// whether the implementation actually generalizes the rule to a nested
/// fact, not just the bare-fact case every worked example used. Built by
/// hand against the AST (see module doc comment for why).
#[test]
fn undeclared_predicate_inside_explicit_produces_block_is_still_an_error() {
    fn dummy_node(seg: &str) -> ast::NodeRef {
        ast::NodeRef {
            segments: seg.split('/').map(str::to_string).collect(),
            span: Span::new(""),
        }
    }

    let commit = ast::CommitStmt {
        predicate_verb: "mints".to_string(),
        items: vec![
            CommitItem::Declare(DeclareStmt {
                kind: DeclKind::Relation,
                ident: "opensTo".to_string(),
                span: Span::new(""),
            }),
            CommitItem::Produces(ast::ProducesBlock {
                facts: vec![
                    FactStmt {
                        subject: dummy_node("room/42"),
                        predicate: PredicateRef::Ident("opensTo".to_string()),
                        value: Value::Node(dummy_node("room/43")),
                        span: Span::new(""),
                    },
                    FactStmt {
                        subject: dummy_node("room/42"),
                        predicate: PredicateRef::Ident("dampness".to_string()),
                        value: Value::Literal(Literal::Number("0.4".to_string())),
                        span: Span::new(""),
                    },
                ],
                span: Span::new(""),
            }),
        ],
        span: Span::new(""),
    };

    let errs = validate_declarations(&commit).expect_err("should be undeclared");
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].predicate, "dampness");
}

/// Not a worked example -- only stated as rule 5 ("Consumes items are
/// skipped entirely"). A consumes-only commit with an undeclared-looking
/// predicate reference must NOT error.
#[test]
fn consumes_only_commit_is_never_checked() {
    let json = r#"{
        "verb": "becomes",
        "consumes": [
            {"kind": "fact",
             "commit": {"uri": "at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789", "cid": "bafyabcxyz"},
             "subject": "room/42",
             "predicate": "locked"}
        ]
    }"#;
    assert_eq!(validate_json(json), Ok(()));
}
