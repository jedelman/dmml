//! Two-tier predicate validation (`SPEC.md` SS3, SS10's "self-declaration
//! ordering" note): every predicate used in a `produces` position is
//! either the closed `rdf:type`/`a` shorthand, or must be self-declared
//! (`declare relation <ident>` / `declare attribute <ident>`) *somewhere*
//! in the same commit -- order-independent within that commit (the real
//! engine's `validate_self_declared` is "commit-batch-sensitive, not
//! line-order-sensitive": it collects every declaration first, then
//! checks every use against the whole collected set).
//!
//! Genuinely combinatorial, unlike `crate::lower`'s flat walk over a
//! closed set of 6 AST variants: this needs a first pass building an
//! unbounded set of declared idents, then a second pass checking an
//! unbounded number of predicate uses against that set -- not
//! enumerable by a handful of worked examples the way lowering was.
//!
//! Deliberately scoped to a single commit's own `declare`/fact items
//! (the "declare-then-assert, one commit" convenience `SPEC.md` SS10
//! documents), not the full cross-commit-history self-declaration rule
//! SPEC.SCRATCH.md SS3 also allows ("declared... in the same commit or
//! an earlier one") -- that needs access to a repo's prior commit
//! history, which a single parsed `Document` doesn't carry. A real
//! resolve-time validator would need that broader check; this is the
//! self-contained, single-commit subset of it.

use crate::ast;
use crate::lower::{ConsumeRef, LoweredCommit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndeclaredPredicate {
    /// The predicate identifier that was used without being declared.
    pub predicate: String,
    /// Span of the fact statement that used it.
    pub span: ast::Span,
}

/// Checks every fact in `commit` (whether a bare `CommitItem::Fact`/
/// `CommitItem::Declare` or one inside an explicit `produces { }` block --
/// both contribute identically, same as lowering) against the two-tier
/// rule. Returns every undeclared use found, in document order; `Ok(())`
/// if every predicate used was either `rdf:type` or declared somewhere
/// in `commit.items`.
///
/// `FactConsume` predicates (inside a `consumes { }` block) are NOT
/// checked here -- a consume references an already-established fact from
/// prior history, not a new assertion, so it isn't subject to this
/// commit's own self-declaration requirement.
/// Datalog-backed as of the cutover that added `crate::datalog_validate`
/// -- a real fixpoint (`Undeclared(idx) <- UsedAt(idx, pred),
/// !Declared(pred)`) replaced the hand-rolled two-pass check that used
/// to live here, proven equivalent by that module's own tests
/// (including the document-order requirement `validate_spec_examples.
/// rs`'s `example_5_multiple_undeclared_reported_in_order` depends on).
pub fn validate_declarations(commit: &ast::CommitStmt) -> Result<(), Vec<UndeclaredPredicate>> {
    crate::datalog_validate::validate_declarations_with_spans(commit)
}

/// See `VALIDATION_SPEC.md`'s "Same-repo `consumes` structural
/// validation" section for the full rule set and worked examples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossRepoConsume {
    pub index: usize,
    pub foreign_did: String,
}

fn did_of_at_uri(at_uri: &str) -> Option<&str> {
    let rest = at_uri.strip_prefix("at://")?;
    let segment = rest.split('/').next()?;
    if segment.is_empty() {
        None
    } else {
        Some(segment)
    }
}

pub fn validate_same_repo_consumes(
    commit: &LoweredCommit,
    authoring_did: &str,
) -> Result<(), Vec<CrossRepoConsume>> {
    let mut violations = Vec::new();
    for (index, consume_ref) in commit.consumes.iter().enumerate() {
        let uri = match consume_ref {
            ConsumeRef::Strong(sr) => &sr.uri,
            ConsumeRef::Fact(fr) => &fr.commit.uri,
        };
        let did_opt = did_of_at_uri(uri);
        let foreign_did = match did_opt {
            None => "<unparseable>".to_string(),
            Some(did) if did != authoring_did => did.to_string(),
            Some(_) => continue,
        };
        violations.push(CrossRepoConsume { index, foreign_did });
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// The tie-in to the already-proven contract: computes
/// `is_cross_repo_consume` from real data via `validate_same_repo_
/// consumes`, then calls (never reimplements) `dmml::resolver::
/// cross_repo_commit_valid`.
pub fn commit_is_valid(commit: &LoweredCommit, authoring_did: &str, declarations_ok: bool) -> bool {
    let is_cross_repo_consume = validate_same_repo_consumes(commit, authoring_did).is_err();
    crate::resolver::cross_repo_commit_valid(is_cross_repo_consume, declarations_ok)
}
