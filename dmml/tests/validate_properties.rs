//! Property test for `validate_declarations`, stressing the combinatorics
//! the 7 hand-picked examples in validate_spec_examples.rs can't reach:
//! many declares and facts, in random order, random overlap between
//! declared and used predicate names. Builds `ast::CommitStmt` values
//! directly (bypassing the parser -- this is testing the two-pass
//! algorithm itself, not parsing) and checks the result against an
//! independent reference computation over the same generated data,
//! not against the implementation under test.

use dmml::ast::{
    CommitItem, CommitStmt, DeclKind, DeclareStmt, FactStmt, Literal, NodeRef, PredicateRef, Span,
    Value,
};
use dmml::validate::validate_declarations;
use proptest::prelude::*;
use std::collections::HashSet;

fn dummy_span() -> Span {
    Span::new("")
}

fn dummy_node_ref() -> NodeRef {
    NodeRef {
        segments: vec!["n".to_string()],
        span: dummy_span(),
    }
}

/// One generated commit item: either a declare of one of a small fixed
/// set of idents, or a fact using rdf:type or one of that same small
/// set -- small alphabet on purpose, to force real overlap between
/// declared and used names across many random orderings.
#[derive(Debug, Clone)]
enum GenItem {
    Declare(String),
    FactRdfType,
    FactIdent(String),
}

fn gen_item() -> impl Strategy<Value = GenItem> {
    let ident = prop_oneof![
        Just("p0".to_string()),
        Just("p1".to_string()),
        Just("p2".to_string()),
        Just("p3".to_string()),
    ];
    prop_oneof![
        ident.clone().prop_map(GenItem::Declare),
        Just(GenItem::FactRdfType),
        ident.prop_map(GenItem::FactIdent),
    ]
}

fn to_ast_item(g: &GenItem) -> CommitItem {
    match g {
        GenItem::Declare(ident) => CommitItem::Declare(DeclareStmt {
            kind: DeclKind::Relation,
            ident: ident.clone(),
            span: dummy_span(),
        }),
        GenItem::FactRdfType => CommitItem::Fact(FactStmt {
            subject: dummy_node_ref(),
            predicate: PredicateRef::RdfType,
            value: Value::Literal(Literal::Boolean(true)),
            span: dummy_span(),
        }),
        GenItem::FactIdent(ident) => CommitItem::Fact(FactStmt {
            subject: dummy_node_ref(),
            predicate: PredicateRef::Ident(ident.clone()),
            value: Value::Literal(Literal::Boolean(true)),
            span: dummy_span(),
        }),
    }
}

proptest! {
    #[test]
    fn matches_independent_reference_computation(items in prop::collection::vec(gen_item(), 0..20)) {
        let commit = CommitStmt {
            predicate_verb: "mints".to_string(),
            items: items.iter().map(to_ast_item).collect(),
            refs: std::collections::HashMap::new(),
            span: dummy_span(),
        };

        // Independent reference: declared set from any Declare, then every
        // FactIdent not in that set is an expected error, in order.
        let declared: HashSet<&str> = items
            .iter()
            .filter_map(|g| match g {
                GenItem::Declare(ident) => Some(ident.as_str()),
                _ => None,
            })
            .collect();
        let expected_undeclared: Vec<&str> = items
            .iter()
            .filter_map(|g| match g {
                GenItem::FactIdent(ident) if !declared.contains(ident.as_str()) => {
                    Some(ident.as_str())
                }
                _ => None,
            })
            .collect();

        let result = validate_declarations(&commit);

        if expected_undeclared.is_empty() {
            prop_assert_eq!(result, Ok(()));
        } else {
            let errs = result.expect_err("expected undeclared predicates");
            let got: Vec<&str> = errs.iter().map(|e| e.predicate.as_str()).collect();
            prop_assert_eq!(got, expected_undeclared);
        }
    }
}
