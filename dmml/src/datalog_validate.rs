//! A real Datalog replacement for `validate::validate_declarations`'s
//! hand-rolled two-pass check -- that module's own doc comment already
//! named the shape precisely: "genuinely combinatorial... a first pass
//! building an unbounded set of declared idents, then a second pass
//! checking an unbounded number of predicate uses against that set,"
//! which is exactly `Declared(p)`/`Used(p)`/`Undeclared(p) <- Used(p),
//! !Declared(p)` -- one stratum, no cycle risk at all (`Declared` is
//! pure `@input`, never derived from anything `Undeclared` could
//! feed back into).
//!
//! `validate::validate_same_repo_consumes` and `validate::commit_is_valid`
//! were reviewed alongside this and deliberately NOT touched:
//! `validate_same_repo_consumes` is per-item string parsing (splitting
//! an `at://` URI, comparing a DID) with no derivation over a fact set
//! at all -- there's nothing here for Datalog to do that a `for` loop
//! doesn't already do just as clearly. `commit_is_valid` calls (never
//! reimplements) `resolver::cross_repo_commit_valid`, one of the five
//! L3-certified Thermite/Verus gate functions this whole review has
//! consistently left alone -- see `dmml-runtime`'s own architecture
//! review notes for why converting a formally-proven function to
//! Datalog would be a downgrade in assurance, not an improvement.

use std::collections::{HashMap, HashSet};

use crate::ast;

/// Interns predicate identifier strings to small `u32` symbols, since
/// crepe's fact fields must be `Copy`. Local to this module -- see
/// `datalog_guard.rs`'s own copy for why a tiny, single-file-local
/// interner isn't worth sharing here (unlike `dmml-runtime`'s three-
/// module, byte-identical duplication that `datalog_support` actually
/// fixed).
#[derive(Default)]
struct SymbolTable {
    by_str: HashMap<String, u32>,
}

impl SymbolTable {
    fn intern(&mut self, s: &str) -> u32 {
        let next = self.by_str.len() as u32;
        *self.by_str.entry(s.to_string()).or_insert(next)
    }
}

crepe::crepe! {
    @input
    struct Declared(u32); // (predicate)
    @input
    struct UsedAt(u32, u32); // (use_index, predicate)

    @output
    struct Undeclared(u32); // (use_index)

    Undeclared(idx) <- UsedAt(idx, pred), !Declared(pred);
}

/// Drop-in equivalent of `validate::validate_declarations`: every
/// predicate used in a `produces` position (bare fact or inside an
/// explicit `produces {}` block, `consumes {}` skipped entirely) must be
/// either the closed `rdf:type`/`a` shorthand or self-declared somewhere
/// in `commit.items`, order-independent for declarations, but errors are
/// still returned in the exact document order the uses appeared in --
/// `validate_spec_examples.rs`'s own `example_5_multiple_undeclared_
/// reported_in_order` depends on this, so the Datalog-derived (and
/// therefore unordered) `Undeclared` set is re-walked against the
/// original ordered use-list afterward, never returned as-is. Returns
/// `validate::UndeclaredPredicate` (predicate + span) directly, the
/// exact type `validate::validate_declarations` itself returns, since
/// that function now delegates straight to this one.
pub fn validate_declarations_with_spans(
    commit: &ast::CommitStmt,
) -> Result<(), Vec<crate::validate::UndeclaredPredicate>> {
    let mut sym = SymbolTable::default();
    let mut runtime = Crepe::new();

    for item in &commit.items {
        if let ast::CommitItem::Declare(declare_stmt) = item {
            runtime.extend([Declared(sym.intern(&declare_stmt.ident))]);
        }
    }

    // Every fact use, in exact document order -- the same walk
    // `validate_declarations`'s own second pass does (bare facts and
    // facts nested in an explicit `produces {}` block; `consumes {}`
    // skipped entirely, matching rule 5).
    let mut uses: Vec<&ast::FactStmt> = Vec::new();
    for item in &commit.items {
        match item {
            ast::CommitItem::Fact(fact_stmt) => uses.push(fact_stmt),
            ast::CommitItem::Produces(produces_block) => {
                uses.extend(produces_block.facts.iter());
            }
            _ => {}
        }
    }

    for (idx, fact_stmt) in uses.iter().enumerate() {
        if let ast::PredicateRef::Ident(s) = &fact_stmt.predicate {
            runtime.extend([UsedAt(idx as u32, sym.intern(s))]);
        }
    }

    let (undeclared,) = runtime.run();
    let undeclared_indices: HashSet<u32> = undeclared.into_iter().map(|Undeclared(i)| i).collect();

    let errors: Vec<crate::validate::UndeclaredPredicate> = uses
        .iter()
        .enumerate()
        .filter(|(idx, _)| undeclared_indices.contains(&(*idx as u32)))
        .filter_map(|(_, fact_stmt)| match &fact_stmt.predicate {
            ast::PredicateRef::Ident(s) => Some(crate::validate::UndeclaredPredicate {
                predicate: s.clone(),
                span: fact_stmt.span.clone(),
            }),
            ast::PredicateRef::RdfType => None,
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::validate_declarations as hand_rolled_validate_declarations;

    fn first_commit(json: &str) -> ast::CommitStmt {
        crate::from_json::commit_from_json(json).expect("should build")
    }

    fn assert_agrees(commit: &ast::CommitStmt) -> Result<(), Vec<String>> {
        let ours = validate_declarations_with_spans(commit);
        let theirs = hand_rolled_validate_declarations(commit);
        match (&ours, &theirs) {
            (Ok(()), Ok(())) => {}
            (Err(a), Err(b)) => {
                let a_preds: Vec<&str> = a.iter().map(|e| e.predicate.as_str()).collect();
                let b_preds: Vec<&str> = b.iter().map(|e| e.predicate.as_str()).collect();
                assert_eq!(
                    a_preds, b_preds,
                    "Datalog and hand-rolled validate_declarations disagree on order/content"
                );
            }
            _ => panic!("verdicts disagree: ours={ours:?} theirs={theirs:?}"),
        }
        ours.map_err(|errs| errs.into_iter().map(|e| e.predicate).collect())
    }

    #[test]
    fn declared_before_use_is_ok() {
        let commit = first_commit(
            r#"{"verb": "mints", "declares": [{"kind": "relation", "name": "opensTo"}],
                "facts": [{"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}}]}"#,
        );
        assert!(assert_agrees(&commit).is_ok());
    }

    #[test]
    fn declared_after_use_is_ok_order_independent() {
        let commit = first_commit(
            r#"{"verb": "mints",
                "facts": [{"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}}],
                "declares": [{"kind": "relation", "name": "opensTo"}]}"#,
        );
        assert!(assert_agrees(&commit).is_ok());
    }

    #[test]
    fn never_declared_is_an_error() {
        let commit = first_commit(
            r#"{"verb": "mints",
                "facts": [{"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}}]}"#,
        );
        assert_eq!(assert_agrees(&commit).unwrap_err(), vec!["opensTo".to_string()]);
    }

    #[test]
    fn rdf_type_never_needs_declaring() {
        let commit = first_commit(
            r#"{"verb": "mints",
                "facts": [{"subject": "room/42", "predicate": "a", "object": {"kind": "node", "value": "Room"}}]}"#,
        );
        assert!(assert_agrees(&commit).is_ok());
    }

    /// The real reason this needed its own module, not just a set-
    /// membership check: multiple undeclared uses must be reported in
    /// exact document order.
    #[test]
    fn multiple_undeclared_reported_in_order() {
        let commit = first_commit(
            r#"{"verb": "mints",
                "facts": [
                    {"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}},
                    {"subject": "room/42", "predicate": "dampness", "object": {"kind": "number", "value": "0.4"}}
                ]}"#,
        );
        assert_eq!(
            assert_agrees(&commit).unwrap_err(),
            vec!["opensTo".to_string(), "dampness".to_string()]
        );
    }

    /// JSON authoring never produces an explicit `CommitItem::Produces`
    /// (bare facts are the sole JSON shape -- see `from_json`'s own doc
    /// comment), so this checks the Datalog path still generalizes to
    /// that AST shape directly, built by hand.
    #[test]
    fn undeclared_inside_explicit_produces_block_is_still_an_error() {
        let commit = ast::CommitStmt {
            predicate_verb: "mints".to_string(),
            items: vec![ast::CommitItem::Produces(ast::ProducesBlock {
                facts: vec![ast::FactStmt {
                    subject: ast::NodeRef {
                        segments: vec!["room".to_string(), "42".to_string()],
                        span: ast::Span::new(""),
                    },
                    predicate: ast::PredicateRef::Ident("opensTo".to_string()),
                    value: ast::Value::Node(ast::NodeRef {
                        segments: vec!["room".to_string(), "43".to_string()],
                        span: ast::Span::new(""),
                    }),
                    span: ast::Span::new(""),
                }],
                span: ast::Span::new(""),
            })],
            span: ast::Span::new(""),
        };
        assert_eq!(assert_agrees(&commit).unwrap_err(), vec!["opensTo".to_string()]);
    }

    #[test]
    fn consumes_only_commit_is_never_checked() {
        let commit = first_commit(
            r#"{"verb": "becomes", "consumes": [
                {"kind": "fact",
                 "commit": {"uri": "at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789", "cid": "bafyabcxyz"},
                 "subject": "room/42",
                 "predicate": "locked"}
            ]}"#,
        );
        assert!(assert_agrees(&commit).is_ok());
    }
}
