//! JSON -> `ast::*` construction. This is DMML's *only* authoring
//! surface: there used to be a hand-written source language with its own
//! lexer/recursive-descent parser, retired once it became clear nothing
//! actually hand-writes DMML source text -- only agents author commits,
//! and JSON (not a bespoke text grammar) is what a tool-calling agent
//! actually produces reliably. The three functions here
//! (`commit_from_json`, `machine_from_json`, `reference_from_json`)
//! deserialize straight into `ast::CommitStmt`/`ast::MachineStmt`/
//! `ast::ReferenceStmt` -- no intermediate source text is ever rendered
//! or re-parsed. Everything downstream (`validate_declarations`, `lower`,
//! `interpret`, the `datalog_*` modules) still runs on the resulting AST
//! exactly as it always has; this module only builds that AST from a
//! different starting shape than before.
//!
//! Design rules the JSON shapes below all follow, so a tool-calling
//! agent's output is checked at the API boundary before it ever reaches
//! this code:
//! - One discriminant field name everywhere a shape is tagged: `kind`.
//! - Every tagged variant is distinguishable by that one field alone,
//!   never by which other fields happen to be present.
//! - Omitting a field always means the same thing every time it's
//!   omittable (an empty list, or -- for `FactConsumeInput::object` --
//!   `FactRef`'s existing wildcard semantics); `null` is never sent or
//!   expected.
//! - Node references and predicates stay plain strings (`"room/42"`),
//!   not decomposed structs -- validated after the fact via the same
//!   shape checks the old text lexer enforced, not by asking a model to
//!   produce a different shape than the prose it already writes.
//!
//! `ast::Span` is a JSON Pointer (RFC 6901) into the request payload this
//! AST node came from, e.g. `/facts/2/predicate` -- the direct
//! replacement for the byte-range-into-source-text span the old text
//! parser produced. Built by hand at each construction site below (never
//! recovered from a general `serde_path_to_error`-style mechanism),
//! since the indices are already in hand while walking the input.

use crate::ast;
use serde::Deserialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub pointer: String,
    pub message: String,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.pointer, self.message)
    }
}

#[derive(Debug)]
pub enum FromJsonError {
    /// The request body wasn't valid JSON, or didn't match the expected
    /// input shape at all (wrong type, missing required field).
    Json(serde_json::Error),
    /// The JSON was shaped correctly, but a value inside it isn't valid
    /// DMML content (a malformed identifier, node reference, or at-uri;
    /// an empty commit).
    Invalid(JsonError),
}

impl fmt::Display for FromJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FromJsonError::Json(e) => write!(f, "invalid JSON: {e}"),
            FromJsonError::Invalid(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for FromJsonError {}

fn invalid(pointer: impl Into<String>, message: impl Into<String>) -> FromJsonError {
    FromJsonError::Invalid(JsonError {
        pointer: pointer.into(),
        message: message.into(),
    })
}

/// A bare `ident`: letter-led, otherwise alphanumeric/underscore. The
/// lexical class a `verb`, a `declare` name, and a plain (non-`"a"`)
/// predicate all occupy.
fn is_valid_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => chars.all(|c| c.is_ascii_alphanumeric() || c == '_'),
        _ => false,
    }
}

/// `seg_piece`: `ident | number`, where `number` here is the digit-led
/// form only (no leading `-`, no decimal part).
fn is_valid_seg_piece(s: &str) -> bool {
    is_valid_ident(s) || (!s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
}

/// `node_ref`: `segment , { "/" , segment }` where `segment = seg_piece ,
/// { "." , seg_piece }` -- e.g. `room/42`, `key/7`, `room/42.reach`.
fn is_valid_node_ref(s: &str) -> bool {
    !s.is_empty()
        && s.split('/')
            .all(|segment| !segment.is_empty() && segment.split('.').all(is_valid_seg_piece))
}

fn check_ident(pointer: &str, value: &str) -> Result<(), FromJsonError> {
    if is_valid_ident(value) {
        Ok(())
    } else {
        Err(invalid(pointer, format!("{value:?} is not a valid identifier")))
    }
}

fn node_ref(pointer: &str, value: &str) -> Result<ast::NodeRef, FromJsonError> {
    if !is_valid_node_ref(value) {
        return Err(invalid(pointer, format!("{value:?} is not a valid node reference")));
    }
    Ok(ast::NodeRef {
        segments: value.split('/').map(str::to_string).collect(),
        span: ast::Span::new(pointer),
    })
}

/// `predicate_ref = "a" | ident` -- the one place a bare, non-`ident`
/// token is legal on its own.
fn predicate_ref(pointer: &str, value: &str) -> Result<ast::PredicateRef, FromJsonError> {
    if value == "a" {
        Ok(ast::PredicateRef::RdfType)
    } else {
        check_ident(pointer, value)?;
        Ok(ast::PredicateRef::Ident(value.to_string()))
    }
}

/// `"at://" , did , "/" , nsid , "/" , rkey` -- atproto's own AT-URI
/// syntax. Requires exactly three non-empty slash-delimited segments
/// after the `at://` prefix.
fn at_uri(pointer: &str, raw: &str) -> Result<ast::AtUri, FromJsonError> {
    let rest = raw
        .strip_prefix("at://")
        .ok_or_else(|| invalid(pointer, format!("{raw:?} is missing the at:// prefix")))?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) {
        return Err(invalid(
            pointer,
            format!("{raw:?} must be at://did/nsid/rkey, found {} segment(s)", parts.len()),
        ));
    }
    Ok(ast::AtUri {
        raw: raw.to_string(),
        did: parts[0].to_string(),
        nsid: parts[1].to_string(),
        rkey: parts[2].to_string(),
    })
}

// ---------------------------------------------------------------------
// Shared shapes
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct StrongRefInput {
    pub uri: String,
    pub cid: String,
}

fn strong_ref(pointer: &str, input: &StrongRefInput) -> Result<ast::StrongRef, FromJsonError> {
    Ok(ast::StrongRef {
        uri: at_uri(&format!("{pointer}/uri"), &input.uri)?,
        cid: input.cid.clone(),
        span: ast::Span::new(pointer),
    })
}

/// Mirrors `dmml::lower::TripleValue`'s shape: an agent picks which kind
/// of object a fact (or a `FactConsume`'s object) has. `kind` is the one
/// discriminant; each variant's own field (`value`) is the only other
/// field present.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ObjectInput {
    Node { value: String },
    Str { value: String },
    Number { value: String },
    Boolean { value: bool },
}

fn value(pointer: &str, input: &ObjectInput) -> Result<ast::Value, FromJsonError> {
    Ok(match input {
        ObjectInput::Node { value } => ast::Value::Node(node_ref(&format!("{pointer}/value"), value)?),
        ObjectInput::Str { value } => ast::Value::Literal(ast::Literal::String(value.clone())),
        ObjectInput::Number { value } => ast::Value::Literal(ast::Literal::Number(value.clone())),
        ObjectInput::Boolean { value } => ast::Value::Literal(ast::Literal::Boolean(*value)),
    })
}

// ---------------------------------------------------------------------
// CommitInput
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeclareKind {
    Relation,
    Attribute,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeclareInput {
    pub kind: DeclareKind,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FactInput {
    pub subject: String,
    pub predicate: String,
    pub object: ObjectInput,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FactConsumeInput {
    pub commit: StrongRefInput,
    pub subject: String,
    pub predicate: String,
    /// Omitted entirely means `FactRef`'s existing wildcard semantics
    /// (every triple asserted for `(subject, predicate)`) -- never sent
    /// as an explicit `null`.
    #[serde(default)]
    pub object: Option<ObjectInput>,
}

/// `kind: "strong"` for a whole-commit reference, `kind: "fact"` for a
/// `FactRef` -- the one discriminant, same convention as `ObjectInput`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ConsumeEntryInput {
    Strong(StrongRefInput),
    Fact(FactConsumeInput),
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitInput {
    pub verb: String,
    #[serde(default)]
    pub declares: Vec<DeclareInput>,
    #[serde(default)]
    pub facts: Vec<FactInput>,
    #[serde(default)]
    pub consumes: Vec<ConsumeEntryInput>,
    pub via: Option<StrongRefInput>,
    #[serde(rename = "respondsTo")]
    pub responds_to: Option<StrongRefInput>,
}

/// Builds an `ast::CommitStmt` directly from a `CommitInput` -- no source
/// text is ever produced. `facts` entries become bare `CommitItem::Fact`
/// items (the "implicit produces block" sugar `CommitItem::Fact`'s own
/// doc comment describes), matching how every existing JSON-authored
/// commit has always been shaped; there is no JSON equivalent of an
/// explicit `produces { }` block, since the two forms are semantically
/// identical and JSON never needed the distinction text authoring used
/// to allow.
pub fn commit_stmt_from_input(input: &CommitInput) -> Result<ast::CommitStmt, FromJsonError> {
    check_ident("/verb", &input.verb)?;
    if input.facts.is_empty()
        && input.consumes.is_empty()
        && input.via.is_none()
        && input.responds_to.is_none()
    {
        return Err(invalid("", "commit has no facts, consumes, via, or respondsTo"));
    }

    let mut items = Vec::new();

    for (i, decl) in input.declares.iter().enumerate() {
        let pointer = format!("/declares/{i}");
        check_ident(&format!("{pointer}/name"), &decl.name)?;
        items.push(ast::CommitItem::Declare(ast::DeclareStmt {
            kind: match decl.kind {
                DeclareKind::Relation => ast::DeclKind::Relation,
                DeclareKind::Attribute => ast::DeclKind::Attribute,
            },
            ident: decl.name.clone(),
            span: ast::Span::new(pointer),
        }));
    }

    for (i, fact) in input.facts.iter().enumerate() {
        let pointer = format!("/facts/{i}");
        items.push(ast::CommitItem::Fact(ast::FactStmt {
            subject: node_ref(&format!("{pointer}/subject"), &fact.subject)?,
            predicate: predicate_ref(&format!("{pointer}/predicate"), &fact.predicate)?,
            value: value(&format!("{pointer}/object"), &fact.object)?,
            span: ast::Span::new(pointer),
        }));
    }

    if !input.consumes.is_empty() {
        let mut entries = Vec::new();
        for (i, entry) in input.consumes.iter().enumerate() {
            let pointer = format!("/consumes/{i}");
            entries.push(match entry {
                ConsumeEntryInput::Strong(sr) => ast::ConsumeEntry::Strong(strong_ref(&pointer, sr)?),
                ConsumeEntryInput::Fact(fc) => ast::ConsumeEntry::Fact(ast::FactConsume {
                    commit: strong_ref(&format!("{pointer}/commit"), &fc.commit)?,
                    subject: node_ref(&format!("{pointer}/subject"), &fc.subject)?,
                    predicate: {
                        check_ident(&format!("{pointer}/predicate"), &fc.predicate)?;
                        fc.predicate.clone()
                    },
                    object: fc.object.as_ref().map(|o| value(&format!("{pointer}/object"), o)).transpose()?,
                    span: ast::Span::new(pointer),
                }),
            });
        }
        items.push(ast::CommitItem::Consumes(ast::ConsumesBlock {
            entries,
            span: ast::Span::new("/consumes"),
        }));
    }

    if let Some(via) = &input.via {
        items.push(ast::CommitItem::Via(strong_ref("/via", via)?));
    }
    if let Some(responds_to) = &input.responds_to {
        items.push(ast::CommitItem::RespondsTo(strong_ref("/respondsTo", responds_to)?));
    }

    Ok(ast::CommitStmt {
        predicate_verb: input.verb.clone(),
        items,
        span: ast::Span::new(""),
    })
}

/// Parses `json` as a `CommitInput` and builds an `ast::CommitStmt` in
/// one step -- the entry point an agent-facing caller actually wants:
/// hand it whatever JSON a tool call returned, get back either a real
/// AST node or a specific, localized error (which half failed: the JSON
/// shape itself, or a value inside it).
pub fn commit_from_json(json: &str) -> Result<ast::CommitStmt, FromJsonError> {
    let input: CommitInput = serde_json::from_str(json).map_err(FromJsonError::Json)?;
    commit_stmt_from_input(&input)
}

// ---------------------------------------------------------------------
// MachineInput
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct StateInput {
    pub ident: String,
}

/// `kind` is the one discriminant; `value` is present for every variant
/// except `self` (which needs no payload -- it always means the
/// machine's own node).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PatternTermInput {
    #[serde(rename = "self")]
    SelfRef,
    Param { value: String },
    Var { value: String },
    Node { value: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct PatternHopInput {
    pub predicate: String,
    pub term: PatternTermInput,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExistsInput {
    pub anchor: PatternTermInput,
    pub hops: Vec<PatternHopInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GuardInput {
    #[serde(default)]
    pub negated: bool,
    pub exists: ExistsInput,
}

/// `kind: "assert" | "retract"`, paired with the state ident the effect
/// names -- matches the grammar's own value-only effect shape
/// (`retract <ident>` / `assert <ident>`, always implicitly `(self,
/// "state", <ident>)`).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum EffectInput {
    Assert { ident: String },
    Retract { ident: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransitionInput {
    pub ident: String,
    #[serde(default)]
    pub params: Vec<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    #[serde(default)]
    pub guards: Vec<GuardInput>,
    #[serde(default)]
    pub effects: Vec<EffectInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MachineInput {
    pub node: String,
    #[serde(default)]
    pub states: Vec<StateInput>,
    #[serde(default)]
    pub transitions: Vec<TransitionInput>,
}

fn pattern_term(pointer: &str, input: &PatternTermInput) -> Result<crate::machine::PatternTerm, FromJsonError> {
    use crate::machine::PatternTerm;
    Ok(match input {
        PatternTermInput::SelfRef => PatternTerm::SelfRef,
        PatternTermInput::Param { value } => {
            check_ident(&format!("{pointer}/value"), value)?;
            PatternTerm::Param(value.clone())
        }
        PatternTermInput::Var { value } => {
            check_ident(&format!("{pointer}/value"), value)?;
            PatternTerm::Var(value.clone())
        }
        PatternTermInput::Node { value } => {
            if !is_valid_node_ref(value) {
                return Err(invalid(format!("{pointer}/value"), format!("{value:?} is not a valid node reference")));
            }
            PatternTerm::Node(value.clone())
        }
    })
}

fn exists_expr(pointer: &str, input: &ExistsInput) -> Result<crate::machine::ExistsExpr, FromJsonError> {
    let anchor = pattern_term(&format!("{pointer}/anchor"), &input.anchor)?;
    if input.hops.is_empty() {
        return Err(invalid(format!("{pointer}/hops"), "a pattern must have at least one hop"));
    }
    let mut hops = Vec::new();
    for (i, hop) in input.hops.iter().enumerate() {
        let hop_pointer = format!("{pointer}/hops/{i}");
        check_ident(&format!("{hop_pointer}/predicate"), &hop.predicate)?;
        hops.push(crate::machine::PatternHop {
            predicate: hop.predicate.clone(),
            term: pattern_term(&format!("{hop_pointer}/term"), &hop.term)?,
        });
    }
    Ok(crate::machine::ExistsExpr {
        pattern: crate::machine::Pattern { anchor, hops },
        span: ast::Span::new(pointer),
    })
}

/// Builds an `ast::MachineStmt` directly from a `MachineInput`. Every
/// ident-shaped field (state names, transition names, params, guard hop
/// predicates, effect targets) is validated as a real DMML identifier
/// before being placed in the AST -- the JSON equivalent of what the old
/// text tokenizer's `RESERVED`-word/character-class checks did, just
/// checking AST-construction-time invariants instead of splicing safety.
pub fn machine_stmt_from_input(input: &MachineInput) -> Result<ast::MachineStmt, FromJsonError> {
    let node = node_ref("/node", &input.node)?;

    let mut states = Vec::new();
    for (i, s) in input.states.iter().enumerate() {
        let pointer = format!("/states/{i}");
        check_ident(&format!("{pointer}/ident"), &s.ident)?;
        states.push(crate::machine::StateDecl {
            ident: s.ident.clone(),
            span: ast::Span::new(pointer),
        });
    }

    let mut transitions = Vec::new();
    for (i, t) in input.transitions.iter().enumerate() {
        let pointer = format!("/transitions/{i}");
        check_ident(&format!("{pointer}/ident"), &t.ident)?;
        for (pi, p) in t.params.iter().enumerate() {
            check_ident(&format!("{pointer}/params/{pi}"), p)?;
        }
        if let Some(from) = &t.from {
            check_ident(&format!("{pointer}/from"), from)?;
        }
        if let Some(to) = &t.to {
            check_ident(&format!("{pointer}/to"), to)?;
        }

        let mut guards = Vec::new();
        for (gi, g) in t.guards.iter().enumerate() {
            let guard_pointer = format!("{pointer}/guards/{gi}");
            guards.push(crate::machine::GuardClause {
                negated: g.negated,
                exists: exists_expr(&format!("{guard_pointer}/exists"), &g.exists)?,
                span: ast::Span::new(guard_pointer),
            });
        }

        let mut effects = Vec::new();
        for (ei, e) in t.effects.iter().enumerate() {
            let effect_pointer = format!("{pointer}/effects/{ei}");
            effects.push(match e {
                EffectInput::Assert { ident } => {
                    check_ident(&format!("{effect_pointer}/ident"), ident)?;
                    crate::machine::Effect::Assert(ident.clone())
                }
                EffectInput::Retract { ident } => {
                    check_ident(&format!("{effect_pointer}/ident"), ident)?;
                    crate::machine::Effect::Retract(ident.clone())
                }
            });
        }

        let has_content = !guards.is_empty() || (t.from.is_some() && t.to.is_some()) || !effects.is_empty();
        if !has_content {
            return Err(invalid(
                &pointer,
                "transition must have at least one of: a guard, a from+to pair, or an effect",
            ));
        }

        transitions.push(crate::machine::TransitionDecl {
            ident: t.ident.clone(),
            params: t.params.clone(),
            from: t.from.clone(),
            to: t.to.clone(),
            guards,
            effects,
            span: ast::Span::new(pointer),
        });
    }

    Ok(ast::MachineStmt {
        node,
        states,
        transitions,
        span: ast::Span::new(""),
    })
}

pub fn machine_from_json(json: &str) -> Result<ast::MachineStmt, FromJsonError> {
    let input: MachineInput = serde_json::from_str(json).map_err(FromJsonError::Json)?;
    machine_stmt_from_input(&input)
}

// ---------------------------------------------------------------------
// ReferenceInput
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceInput {
    pub target: StrongRefInput,
    #[serde(rename = "asName")]
    pub as_name: Option<String>,
}

pub fn reference_stmt_from_input(input: &ReferenceInput) -> Result<ast::ReferenceStmt, FromJsonError> {
    let as_name = input.as_name.as_deref().map(|s| node_ref("/asName", s)).transpose()?;
    Ok(ast::ReferenceStmt {
        target: strong_ref("/target", &input.target)?,
        as_name,
        span: ast::Span::new(""),
    })
}

pub fn reference_from_json(json: &str) -> Result<ast::ReferenceStmt, FromJsonError> {
    let input: ReferenceInput = serde_json::from_str(json).map_err(FromJsonError::Json)?;
    reference_stmt_from_input(&input)
}

// ---------------------------------------------------------------------
// Chat-fence extraction (unchanged: still needed by any caller that lets
// an agent narrate around a JSON commit in one reply)
// ---------------------------------------------------------------------

/// Splits chat-style reply text into (text outside the fence, raw text
/// inside it), or `None` if there's no fenced code block at all. Works
/// entirely in terms of byte offsets into the original `s` (never mixes
/// offsets from a trimmed copy back into it), so the "text after the
/// fence" half can't misalign. Doesn't itself judge whether the fenced
/// text is valid JSON or a real commit -- pair it with `commit_from_json`
/// for that.
pub fn extract_fenced_block(s: &str) -> Option<(String, String)> {
    let open_start = s.find("```")?;
    let after_marker = open_start + 3;
    let body_start = after_marker + s[after_marker..].find('\n').map(|n| n + 1).unwrap_or(0);
    let close_start = body_start + s[body_start..].find("```")?;
    let close_end = close_start + 3;

    let fenced_text = s[body_start..close_start].trim().to_string();
    if fenced_text.is_empty() {
        return None;
    }

    let mut chat_text = format!("{} {}", s[..open_start].trim(), s[close_end..].trim());
    chat_text = chat_text.trim().to_string();
    if chat_text.is_empty() {
        chat_text = "[proposes a commit]".to_string();
    }
    Some((chat_text, fenced_text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::validate_declarations;
    use crate::lower;

    #[test]
    fn renders_and_round_trips_through_the_real_pipeline() {
        let json = r#"{
            "verb": "answers",
            "declares": [
                {"kind": "attribute", "name": "material"},
                {"kind": "relation", "name": "answers"}
            ],
            "facts": [
                {"subject": "key/7", "predicate": "material", "object": {"kind": "node", "value": "material/starIron"}},
                {"subject": "answer/3", "predicate": "answers", "object": {"kind": "node", "value": "question/3"}}
            ]
        }"#;

        let commit = commit_from_json(json).expect("should build");
        validate_declarations(&commit).expect("should self-declare cleanly");
        let lowered = lower::lower_commit(&commit);
        // 2 fact lines + 2 declare_stmt-generated rdf:type triples.
        assert_eq!(lowered.produces.len(), 4);
    }

    #[test]
    fn renders_string_number_and_boolean_objects() {
        let json = r#"{
            "verb": "asserts",
            "declares": [
                {"kind": "attribute", "name": "label"},
                {"kind": "attribute", "name": "count"},
                {"kind": "attribute", "name": "sealed"}
            ],
            "facts": [
                {"subject": "door/1", "predicate": "label", "object": {"kind": "str", "value": "the \"old\" door"}},
                {"subject": "door/1", "predicate": "count", "object": {"kind": "number", "value": "3"}},
                {"subject": "door/1", "predicate": "sealed", "object": {"kind": "boolean", "value": true}}
            ]
        }"#;

        let commit = commit_from_json(json).expect("should build");
        validate_declarations(&commit).expect("should self-declare cleanly");
    }

    #[test]
    fn rejects_an_invalid_identifier() {
        let json = r#"{
            "verb": "answers",
            "declares": [],
            "facts": [
                {"subject": "key/7 not a node", "predicate": "material", "object": {"kind": "node", "value": "oak"}}
            ]
        }"#;

        let err = commit_from_json(json).expect_err("should reject a malformed subject");
        match err {
            FromJsonError::Invalid(JsonError { pointer, .. }) => {
                assert_eq!(pointer, "/facts/0/subject");
            }
            other => panic!("expected an Invalid error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_facts_and_consumes() {
        let json = r#"{"verb": "answers", "declares": [], "facts": []}"#;
        let err = commit_from_json(json).expect_err("should reject an empty commit");
        assert!(matches!(err, FromJsonError::Invalid(_)));
    }

    #[test]
    fn consumes_only_commit_with_no_facts_is_a_valid_pure_retraction() {
        let json = r#"{
            "verb": "becomes",
            "consumes": [
                {"kind": "fact",
                 "commit": {"uri": "at://did:plc:aaaa1111/org.example.commit/xyz789", "cid": "bafyabcxyz"},
                 "subject": "room/42",
                 "predicate": "locked"}
            ]
        }"#;

        let commit = commit_from_json(json).expect("consumes-only commit should build");
        assert!(commit.items.iter().any(|i| matches!(i, ast::CommitItem::Consumes(_))));
        assert!(!commit.items.iter().any(|i| matches!(i, ast::CommitItem::Fact(_))));
        validate_declarations(&commit).expect("no facts means nothing to self-declare");
    }

    #[test]
    fn strong_and_fact_consumes_round_trip() {
        let json = r#"{
            "verb": "becomes",
            "facts": [{"subject": "room/42", "predicate": "sealed", "object": {"kind": "boolean", "value": true}}],
            "declares": [{"kind": "attribute", "name": "sealed"}],
            "consumes": [
                {"kind": "strong", "uri": "at://did:plc:aaaa1111/org.example.commit/1", "cid": "bafy1"},
                {"kind": "fact", "commit": {"uri": "at://did:plc:aaaa1111/org.example.commit/2", "cid": "bafy2"},
                 "subject": "room/42", "predicate": "locked", "object": {"kind": "boolean", "value": true}}
            ],
            "via": {"uri": "at://did:plc:aaaa1111/org.example.commit/3", "cid": "bafy3"},
            "respondsTo": {"uri": "at://did:plc:aaaa1111/org.example.commit/4", "cid": "bafy4"}
        }"#;

        let commit = commit_from_json(json).expect("should build");
        let has_consumes = commit.items.iter().any(|i| matches!(i, ast::CommitItem::Consumes(_)));
        let has_via = commit.items.iter().any(|i| matches!(i, ast::CommitItem::Via(_)));
        let has_responds_to = commit.items.iter().any(|i| matches!(i, ast::CommitItem::RespondsTo(_)));
        assert!(has_consumes && has_via && has_responds_to);
    }

    #[test]
    fn machine_builds_states_and_transitions() {
        let json = r#"{
            "node": "lever/1",
            "states": [{"ident": "up"}, {"ident": "down"}],
            "transitions": [
                {
                    "ident": "pull",
                    "params": ["actor"],
                    "from": "up",
                    "to": "down",
                    "guards": [
                        {"negated": false, "exists": {
                            "anchor": {"kind": "param", "value": "actor"},
                            "hops": [{"predicate": "holds", "term": {"kind": "node", "value": "key/7"}}]
                        }}
                    ],
                    "effects": [{"kind": "assert", "ident": "unlocked"}]
                }
            ]
        }"#;

        let machine = machine_from_json(json).expect("should build");
        assert_eq!(machine.states.len(), 2);
        assert_eq!(machine.transitions.len(), 1);
        assert_eq!(machine.transitions[0].guards.len(), 1);
        assert_eq!(machine.transitions[0].effects.len(), 1);
    }

    #[test]
    fn machine_transition_with_no_content_is_rejected() {
        let json = r#"{
            "node": "lever/1",
            "states": [],
            "transitions": [{"ident": "noop"}]
        }"#;
        let err = machine_from_json(json).expect_err("should reject a contentless transition");
        assert!(matches!(err, FromJsonError::Invalid(_)));
    }

    #[test]
    fn reference_builds() {
        let json = r#"{
            "target": {"uri": "at://did:plc:aaaa1111/org.example.commit/1", "cid": "bafy1"},
            "asName": "lever/old"
        }"#;
        let reference = reference_from_json(json).expect("should build");
        assert!(reference.as_name.is_some());
    }

    #[test]
    fn extracts_a_fenced_block_and_the_surrounding_chat_text() {
        let reply = "Here's my take.\n\n```json\n{\"a\": 1}\n```\n\nWhat do you think?";
        let (chat_text, fenced) = extract_fenced_block(reply).expect("should find a fence");
        assert_eq!(fenced, "{\"a\": 1}");
        assert!(chat_text.contains("Here's my take."));
        assert!(chat_text.contains("What do you think?"));
    }

    #[test]
    fn returns_none_when_there_is_no_fence() {
        assert_eq!(extract_fenced_block("just chatting, no commit here"), None);
    }
}
