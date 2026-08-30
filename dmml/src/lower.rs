//! Lowering: DMML's parsed AST (`crate::ast`) to the flat, engine-shaped
//! record a `Commit` actually needs -- `SPEC.md` SS10's "Formal grammar"
//! section and "Surface syntax" worked examples, made real. Covers
//! `commit_stmt` (`lower_commit`) and `reference_stmt`
//! (`lower_reference`, see `LOWERING_SPEC.md`'s second half); `machine_stmt`
//! stays out of scope (grammar-reserved, not specified -- `SPEC.md`
//! itself hasn't settled it).

use crate::ast;
use serde::Serialize;
use std::collections::HashMap;

/// One `commit { ... }` block, lowered. `produces` is a flat `Vec<Triple>`
/// here rather than serialized N-Quads text, since dmml has no N-Quads
/// writer of its own and doesn't need one to be a useful reference
/// lowering. `refs` replaces the old separate `via`/`responds_to` fields
/// -- see `ast::CommitStmt.refs`'s own doc comment for why every role is
/// a list keyed by an open role name rather than two dedicated
/// `Option<StrongRef>` fields.
#[derive(Debug, Clone, PartialEq)]
pub struct LoweredCommit {
    pub predicate_verb: String,
    pub consumes: Vec<ConsumeRef>,
    pub produces: Vec<Triple>,
    pub refs: HashMap<String, Vec<StrongRef>>,
}

/// `Serialize` derived directly (not a parallel `Wire*` struct, unlike
/// `LoweredCommit`'s own wire shape): a triple's own natural field shape
/// IS what `dmml-substrate-kit`'s `atproto_cid::triple_cid` hashes, no
/// N-Quads-text or DID transformation needed first -- that hashing lives
/// in that separate, substrate-specific crate, not here.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Triple {
    pub subject: String,
    pub predicate: String,
    pub object: TripleValue,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum TripleValue {
    Node(String),
    Number(String),
    Boolean(bool),
    Str(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrongRef {
    pub uri: String,
    pub cid: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConsumeRef {
    Strong(StrongRef),
    Fact(FactRef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FactRef {
    pub commit: StrongRef,
    pub subject: String,
    pub predicate: String,
    pub object: Option<TripleValue>,
}

fn lower_node_ref(n: &ast::NodeRef) -> String {
    n.segments.join("/")
}

fn lower_predicate_ref(p: &ast::PredicateRef) -> String {
    match p {
        ast::PredicateRef::RdfType => "rdf:type".to_string(),
        ast::PredicateRef::Ident(s) => s.clone(),
    }
}

fn lower_value(v: &ast::Value) -> TripleValue {
    match v {
        ast::Value::Node(n) => TripleValue::Node(lower_node_ref(n)),
        ast::Value::Literal(ast::Literal::Number(s)) => TripleValue::Number(s.clone()),
        ast::Value::Literal(ast::Literal::Boolean(b)) => TripleValue::Boolean(*b),
        ast::Value::Literal(ast::Literal::String(s)) => TripleValue::Str(s.clone()),
    }
}

fn lower_strong_ref(sr: &ast::StrongRef) -> StrongRef {
    StrongRef {
        uri: sr.uri.raw.clone(),
        cid: sr.cid.clone(),
    }
}

/// Lowers one parsed `commit { ... }` block. See `dmml/LOWERING_SPEC.md`
/// for the full rule set and worked examples this implements. A bare
/// `Declare`/`Fact` item
/// and a `Produces` block's facts are not distinguished in the output --
/// both contribute to the same flat `produces`, in document order, per
/// SPEC.md's "sugar for implicit produces block" rule.
pub fn lower_commit(commit: &ast::CommitStmt) -> LoweredCommit {
    let mut produces = Vec::new();
    let mut consumes = Vec::new();

    for item in &commit.items {
        match item {
            ast::CommitItem::Declare(d) => {
                let object = match d.kind {
                    ast::DeclKind::Relation => TripleValue::Node("Relation".to_string()),
                    ast::DeclKind::Attribute => TripleValue::Node("Attribute".to_string()),
                };
                produces.push(Triple {
                    subject: d.ident.clone(),
                    predicate: "rdf:type".to_string(),
                    object,
                });
            }
            ast::CommitItem::Fact(f) => {
                produces.push(Triple {
                    subject: lower_node_ref(&f.subject),
                    predicate: lower_predicate_ref(&f.predicate),
                    object: lower_value(&f.value),
                });
            }
            ast::CommitItem::Produces(block) => {
                for fact in &block.facts {
                    produces.push(Triple {
                        subject: lower_node_ref(&fact.subject),
                        predicate: lower_predicate_ref(&fact.predicate),
                        object: lower_value(&fact.value),
                    });
                }
            }
            ast::CommitItem::Consumes(block) => {
                for entry in &block.entries {
                    match entry {
                        ast::ConsumeEntry::Strong(sr) => {
                            consumes.push(ConsumeRef::Strong(lower_strong_ref(sr)));
                        }
                        ast::ConsumeEntry::Fact(fc) => {
                            consumes.push(ConsumeRef::Fact(FactRef {
                                commit: lower_strong_ref(&fc.commit),
                                subject: lower_node_ref(&fc.subject),
                                predicate: fc.predicate.clone(),
                                object: fc.object.as_ref().map(lower_value),
                            }));
                        }
                    }
                }
            }
        }
    }

    let refs = commit
        .refs
        .iter()
        .map(|(role, targets)| (role.clone(), targets.iter().map(lower_strong_ref).collect()))
        .collect();

    LoweredCommit {
        predicate_verb: commit.predicate_verb.clone(),
        consumes,
        produces,
        refs,
    }
}

/// Lowers a top-level `reference` statement. See `LOWERING_SPEC.md`'s
/// "Reference statement lowering" section for the full rule set and
/// worked examples.
pub fn lower_reference(reference: &ast::ReferenceStmt) -> Vec<Triple> {
    match &reference.as_name {
        Some(node_ref) => {
            let subject = node_ref.segments.join("/");
            vec![
                Triple {
                    subject: subject.clone(),
                    predicate: "foreignUri".to_string(),
                    object: TripleValue::Str(reference.target.uri.raw.clone()),
                },
                Triple {
                    subject,
                    predicate: "foreignCid".to_string(),
                    object: TripleValue::Str(reference.target.cid.clone()),
                },
            ]
        }
        None => vec![],
    }
}
