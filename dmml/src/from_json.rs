//! Deterministic JSON -> DMML commit-source rendering.
//!
//! Pure serialization, nothing more: an authoring agent that keeps
//! mis-typing hand-written brace/DMML syntax (unmatched braces, a
//! narrated "(...)" instead of a real fact line, a used-before-declared
//! predicate) describes the same commit as ordinary JSON instead --
//! structurally impossible to get "found `(`"-style syntax errors from,
//! because there's no free-form syntax to get wrong. `render_commit`
//! turns that JSON into real DMML source text; it does NOT itself
//! validate self-declaration or anything semantic -- the rendered
//! source still goes through the ordinary `parse`/`validate_
//! declarations`/`lower` pipeline unchanged, same as hand-written DMML.
//! This module only removes *syntax* risk, not the DMML-first
//! guarantee that the real parser and validator are the single source
//! of truth for whether a commit is actually valid.

use serde::Deserialize;
use std::fmt;

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

/// Mirrors `dmml::lower::TripleValue`'s shape -- an agent picks which
/// kind of object a fact has, the same choice `ANSWER_SYSTEM_PROMPT`-
/// style hand-written rules already ask for (mint a node vs. use a
/// literal), just expressed as data instead of syntax.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ObjectInput {
    Node { value: String },
    Str { value: String },
    Number { value: String },
    Boolean { value: bool },
}

#[derive(Debug, Clone, Deserialize)]
pub struct FactInput {
    pub subject: String,
    pub predicate: String,
    pub object: ObjectInput,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitInput {
    pub verb: String,
    #[serde(default)]
    pub declares: Vec<DeclareInput>,
    pub facts: Vec<FactInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// A verb/subject/predicate/node-value wasn't a valid DMML
    /// identifier or slash-path (e.g. contained whitespace, braces, or
    /// started with a digit) -- rendering it verbatim would either
    /// produce a confusing parse error downstream or, worse, let
    /// stray `{`/`}` in agent-supplied text corrupt the commit's own
    /// structure. Caught here instead, with the offending value named.
    InvalidIdent { field: &'static str, value: String },
    /// No facts at all -- an empty commit is never useful content.
    EmptyFacts,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::InvalidIdent { field, value } => {
                write!(f, "{field} {value:?} is not a valid DMML identifier or slash-path")
            }
            RenderError::EmptyFacts => write!(f, "commit has no facts"),
        }
    }
}

impl std::error::Error for RenderError {}

/// A bare `ident` per `SPEC.md` section 10's EBNF: `letter , { letter |
/// digit | "_" }` -- letter-led, otherwise alphanumeric/underscore.
/// This is the lexical class `predicate_verb`, `declare_stmt`'s name,
/// and `predicate_ref` (a fact's predicate) all occupy -- narrower than
/// `node_ref` below, deliberately: these positions are never digit-led
/// in the real grammar.
fn is_valid_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => chars.all(|c| c.is_ascii_alphanumeric() || c == '_'),
        _ => false,
    }
}

/// A `seg_piece` per the same EBNF: `ident | number`, where `number`
/// here is the digit-led form only (no leading `-`, no decimal part --
/// a node segment like `room/42` is a bare non-negative integer token,
/// never a genuinely negative or fractional one).
fn is_valid_seg_piece(s: &str) -> bool {
    is_valid_ident(s) || (!s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
}

/// A `node_ref` per the same EBNF: `segment , { "/" , segment }` where
/// `segment = seg_piece , { "." , seg_piece }` -- e.g. `room/42`,
/// `key/7`, `room/42.reach`. This is the lexical class a fact's
/// `subject` and any `Node`-valued `object` occupy.
fn is_valid_node_ref(s: &str) -> bool {
    !s.is_empty()
        && s.split('/')
            .all(|segment| !segment.is_empty() && segment.split('.').all(is_valid_seg_piece))
}

fn check_ident(field: &'static str, value: &str) -> Result<(), RenderError> {
    if is_valid_ident(value) {
        Ok(())
    } else {
        Err(RenderError::InvalidIdent {
            field,
            value: value.to_string(),
        })
    }
}

fn check_node_ref(field: &'static str, value: &str) -> Result<(), RenderError> {
    if is_valid_node_ref(value) {
        Ok(())
    } else {
        Err(RenderError::InvalidIdent {
            field,
            value: value.to_string(),
        })
    }
}

/// Escapes a string for a DMML string literal, matching exactly what
/// `lexer.rs::read_string` unescapes on the way back in: `\"` and `\\`
/// are the only two recognized escapes (any other backslash sequence
/// passes through literally), so those are the only two characters
/// this needs to escape here.
fn escape_dmml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn render_object(field: &'static str, object: &ObjectInput) -> Result<String, RenderError> {
    match object {
        ObjectInput::Node { value } => {
            check_node_ref(field, value)?;
            Ok(value.clone())
        }
        ObjectInput::Str { value } => Ok(escape_dmml_string(value)),
        ObjectInput::Number { value } => Ok(value.clone()),
        ObjectInput::Boolean { value } => Ok(value.to_string()),
    }
}

/// `predicate_ref = "a" | ident` -- the one place a bare, non-`ident`
/// token is legal on its own.
fn check_predicate(value: &str) -> Result<(), RenderError> {
    if value == "a" {
        Ok(())
    } else {
        check_ident("predicate", value)
    }
}

/// Renders a `CommitInput` into real DMML commit source text. Every
/// identifier-shaped field (verb, declare names, subjects, predicates,
/// node-valued objects) is validated as a real DMML identifier/slash-
/// path before being written into the source string -- this is what
/// keeps agent-supplied content from corrupting the commit's own
/// structure. Nothing here checks self-declaration, guard semantics,
/// or anything the real `validate_declarations`/`parse` pipeline
/// already checks; callers still run the returned string through that
/// pipeline exactly as they would hand-written source.
pub fn render_commit(input: &CommitInput) -> Result<String, RenderError> {
    check_ident("verb", &input.verb)?;
    if input.facts.is_empty() {
        return Err(RenderError::EmptyFacts);
    }

    let mut out = format!("commit {} {{\n", input.verb);
    for decl in &input.declares {
        check_ident("declare name", &decl.name)?;
        let keyword = match decl.kind {
            DeclareKind::Relation => "relation",
            DeclareKind::Attribute => "attribute",
        };
        out.push_str(&format!("  declare {keyword} {}\n", decl.name));
    }
    if !input.declares.is_empty() {
        out.push('\n');
    }
    for fact in &input.facts {
        check_node_ref("subject", &fact.subject)?;
        check_predicate(&fact.predicate)?;
        let object = render_object("object", &fact.object)?;
        out.push_str(&format!("  {} {} {}\n", fact.subject, fact.predicate, object));
    }
    out.push_str("}\n");
    Ok(out)
}

/// Splits chat-style reply text into (text outside the fence, raw text
/// inside it), or `None` if there's no fenced code block at all. Works
/// entirely in terms of byte offsets into the original `s` (never mixes
/// offsets from a trimmed copy back into it), so the "text after the
/// fence" half can't misalign. Doesn't itself judge whether the fenced
/// text is valid JSON or a real commit -- pair it with
/// `commit_source_from_json` for that. Shared by every caller that lets
/// an agent narrate around a JSON commit in one reply (`chorus_shell.rs`,
/// the server's own steerable enrichment route) instead of duplicating
/// this per caller.
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

#[derive(Debug)]
pub enum FromJsonError {
    Json(serde_json::Error),
    Render(RenderError),
}

impl fmt::Display for FromJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FromJsonError::Json(e) => write!(f, "invalid JSON: {e}"),
            FromJsonError::Render(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for FromJsonError {}

/// Parses a JSON string as a `CommitInput` and renders it to DMML
/// source text in one step -- the entry point an agent-facing caller
/// actually wants: hand it whatever JSON text a model returned, get
/// back either real DMML source or a specific, localized error (which
/// half failed: the JSON shape itself, or an identifier inside it).
pub fn commit_source_from_json(json: &str) -> Result<String, FromJsonError> {
    let input: CommitInput = serde_json::from_str(json).map_err(FromJsonError::Json)?;
    render_commit(&input).map_err(FromJsonError::Render)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::validate_declarations;
    use crate::{ast::TopLevelItem, lower};

    #[test]
    fn renders_and_round_trips_through_the_real_parser() {
        let json = r#"{
            "verb": "answers",
            "declares": [
                {"kind": "attribute", "name": "material"},
                {"kind": "relation", "name": "answers"}
            ],
            "facts": [
                {"subject": "key/7", "predicate": "material", "object": {"type": "node", "value": "material/starIron"}},
                {"subject": "answer/3", "predicate": "answers", "object": {"type": "node", "value": "question/3"}}
            ]
        }"#;

        let src = commit_source_from_json(json).expect("should render");
        let doc = crate::parse(&src).expect("rendered source should parse");
        let commit = doc
            .items
            .iter()
            .find_map(|item| match item {
                TopLevelItem::Commit(c) => Some(c),
                _ => None,
            })
            .expect("a commit item");
        validate_declarations(commit).expect("should self-declare cleanly");
        let lowered = lower::lower_commit(commit);
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
                {"subject": "door/1", "predicate": "label", "object": {"type": "str", "value": "the \"old\" door"}},
                {"subject": "door/1", "predicate": "count", "object": {"type": "number", "value": "3"}},
                {"subject": "door/1", "predicate": "sealed", "object": {"type": "boolean", "value": true}}
            ]
        }"#;

        let src = commit_source_from_json(json).expect("should render");
        let doc = crate::parse(&src).expect("rendered source should parse");
        let commit = doc
            .items
            .iter()
            .find_map(|item| match item {
                TopLevelItem::Commit(c) => Some(c),
                _ => None,
            })
            .expect("a commit item");
        validate_declarations(commit).expect("should self-declare cleanly");
    }

    #[test]
    fn rejects_an_invalid_identifier_rather_than_rendering_it() {
        let json = r#"{
            "verb": "answers",
            "declares": [],
            "facts": [
                {"subject": "key/7 } commit evil { x", "predicate": "material", "object": {"type": "node", "value": "oak"}}
            ]
        }"#;

        let err = commit_source_from_json(json).expect_err("should reject a malformed subject");
        match err {
            FromJsonError::Render(RenderError::InvalidIdent { field, .. }) => {
                assert_eq!(field, "subject");
            }
            other => panic!("expected an InvalidIdent render error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_facts() {
        let json = r#"{"verb": "answers", "declares": [], "facts": []}"#;
        let err = commit_source_from_json(json).expect_err("should reject an empty commit");
        assert!(matches!(err, FromJsonError::Render(RenderError::EmptyFacts)));
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
