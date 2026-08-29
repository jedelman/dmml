//! Structural types and parser for a `machine_stmt` body, per
//! `MACHINE_SPEC.md` (issue #50 Tier 2). `crate::ast::MachineStmt` still
//! carries the body as an opaque, balanced-brace `String` (see that
//! type's own doc comment) -- `parse_machine_body` is the second-pass
//! parser that turns that raw text into these structural types, kept
//! deliberately separate from `crate::parser`'s main recursive-descent
//! pass rather than threading machine-body tokens through the whole
//! document's lexer/parser. Spans in this module are byte offsets into
//! the `body` string itself (post `{`/`}` stripping), not into the
//! whole document -- consistent with `MachineStmt.body` already being a
//! standalone `String`, not a document slice.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDecl {
    pub ident: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDecl {
    pub ident: String,
    pub params: Vec<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub guards: Vec<GuardClause>,
    pub effects: Vec<Effect>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardClause {
    pub negated: bool,
    pub exists: ExistsExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistsExpr {
    pub pattern: Pattern,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub anchor: PatternTerm,
    pub hops: Vec<PatternHop>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternHop {
    pub predicate: String,
    pub term: PatternTerm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternTerm {
    SelfRef,
    Param(String),
    Var(String),
    Node(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Retract(String),
    Assert(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineParseError {
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MachineBody {
    pub states: Vec<StateDecl>,
    pub transitions: Vec<TransitionDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokKind {
    Ident(String),
    Slash,
    Dollar,
    Question,
    Colon,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Tok {
    kind: TokKind,
    span: Span,
}

/// Byte-offset tokenizer for a machine body. Never panics: any byte that
/// doesn't start a recognized token (including non-ASCII/continuation
/// bytes) is a `MachineParseError`, not a crash.
fn tokenize(body: &str) -> Result<Vec<Tok>, MachineParseError> {
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut toks = Vec::new();
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let single = |kind: TokKind, i: usize| Tok {
            kind,
            span: Span { start: i, end: i + 1 },
        };
        match c {
            '/' => {
                toks.push(single(TokKind::Slash, i));
                i += 1;
            }
            '$' => {
                toks.push(single(TokKind::Dollar, i));
                i += 1;
            }
            '?' => {
                toks.push(single(TokKind::Question, i));
                i += 1;
            }
            ':' => {
                toks.push(single(TokKind::Colon, i));
                i += 1;
            }
            ',' => {
                toks.push(single(TokKind::Comma, i));
                i += 1;
            }
            '(' => {
                toks.push(single(TokKind::LParen, i));
                i += 1;
            }
            ')' => {
                toks.push(single(TokKind::RParen, i));
                i += 1;
            }
            '{' => {
                toks.push(single(TokKind::LBrace, i));
                i += 1;
            }
            '}' => {
                toks.push(single(TokKind::RBrace, i));
                i += 1;
            }
            c if c.is_ascii_alphanumeric() || c == '_' => {
                let start = i;
                while i < bytes.len() {
                    let cc = bytes[i] as char;
                    if cc.is_ascii_alphanumeric() || cc == '_' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                toks.push(Tok {
                    kind: TokKind::Ident(body[start..i].to_string()),
                    span: Span { start, end: i },
                });
            }
            other => {
                return Err(MachineParseError {
                    message: format!("unexpected character {:?}", other),
                    span: Span { start: i, end: i + 1 },
                });
            }
        }
    }
    Ok(toks)
}

struct TokenCursor<'a> {
    toks: &'a [Tok],
    pos: usize,
    body_len: usize,
}

impl<'a> TokenCursor<'a> {
    fn new(toks: &'a [Tok], body_len: usize) -> Self {
        TokenCursor { toks, pos: 0, body_len }
    }

    /// Span for error messages when we're at end-of-input: an empty span
    /// at the end of the body.
    fn eof_span(&self) -> Span {
        Span { start: self.body_len, end: self.body_len }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn peek_span(&self) -> Span {
        self.peek().map(|t| t.span).unwrap_or_else(|| self.eof_span())
    }

    fn advance(&mut self) -> Option<&Tok> {
        let t = self.toks.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// True if the next token is `Ident(text)` for the exact given
    /// keyword text (case-sensitive), without consuming it.
    fn peek_is_kw(&self, text: &str) -> bool {
        matches!(self.peek(), Some(Tok { kind: TokKind::Ident(s), .. }) if s == text)
    }

    /// Consumes the next token, requiring it to be `Ident(text)` for the
    /// exact given keyword text. Returns its span, or a `MachineParseError`.
    fn expect_kw(&mut self, text: &str) -> Result<Span, MachineParseError> {
        match self.peek() {
            Some(Tok { kind: TokKind::Ident(s), span }) if s == text => {
                let span = *span;
                self.advance();
                Ok(span)
            }
            Some(Tok { span, .. }) => Err(MachineParseError {
                message: format!("expected keyword {:?}", text),
                span: *span,
            }),
            None => Err(MachineParseError {
                message: format!("expected keyword {:?}, found end of input", text),
                span: self.eof_span(),
            }),
        }
    }

    /// Consumes the next token, requiring it to be an `Ident` that is
    /// NOT one of the reserved keywords listed in `RESERVED`. Returns
    /// the ident text and its span.
    fn expect_ident(&mut self) -> Result<(String, Span), MachineParseError> {
        match self.peek() {
            Some(Tok { kind: TokKind::Ident(s), span }) => {
                if RESERVED.contains(&s.as_str()) {
                    return Err(MachineParseError {
                        message: format!("expected an identifier, found reserved keyword {:?}", s),
                        span: *span,
                    });
                }
                let s = s.clone();
                let span = *span;
                self.advance();
                Ok((s, span))
            }
            Some(Tok { span, .. }) => Err(MachineParseError {
                message: "expected an identifier".to_string(),
                span: *span,
            }),
            None => Err(MachineParseError {
                message: "expected an identifier, found end of input".to_string(),
                span: self.eof_span(),
            }),
        }
    }

    fn expect_punct(&mut self, kind: TokKind) -> Result<Span, MachineParseError> {
        match self.peek() {
            Some(Tok { kind: k, span }) if *k == kind => {
                let span = *span;
                self.advance();
                Ok(span)
            }
            Some(Tok { span, .. }) => Err(MachineParseError {
                message: format!("expected {:?}", kind),
                span: *span,
            }),
            None => Err(MachineParseError {
                message: format!("expected {:?}, found end of input", kind),
                span: self.eof_span(),
            }),
        }
    }
}

/// Keywords that are never valid as a plain `ident` production (state
/// names, transition names, param names, predicate names, node_ref
/// segments) -- matches `MACHINE_SPEC.md`'s "not valid as a plain ident"
/// note. `"self"` is deliberately included: as a `pattern_term` it's the
/// dedicated `self` keyword, never a `node_ref`.
const RESERVED: &[&str] = &[
    "state", "transition", "from", "to", "guard", "effect", "EXISTS", "not", "retract", "assert",
    "self",
];

/// Parses the raw text between (exclusive) a `machine <node_ref> { ... }`
/// block's outer braces -- i.e. `ast::MachineStmt.body` -- into structural
/// `state_decl`/`transition_decl` items per `MACHINE_SPEC.md`'s grammar.
/// Never panics on malformed input; every rejection is a
/// `MachineParseError`.
pub fn parse_machine_body(body: &str) -> Result<MachineBody, MachineParseError> {
    let toks = tokenize(body)?;
    let mut cursor = TokenCursor::new(&toks, body.len());
    parse_machine_body_from(&mut cursor)
}

fn parse_machine_body_from(cursor: &mut TokenCursor) -> Result<MachineBody, MachineParseError> {
    let mut body = MachineBody::default();

    loop {
        match cursor.peek() {
            None => break,
            Some(tok) => {
                if tok.kind == TokKind::Ident("state".to_string()) {
                    let state = parse_state_decl(cursor)?;
                    body.states.push(state);
                } else if tok.kind == TokKind::Ident("transition".to_string()) {
                    let transition = parse_transition_decl(cursor)?;
                    body.transitions.push(transition);
                } else {
                    return Err(MachineParseError {
                        message: "unexpected token: expected 'state' or 'transition'".to_string(),
                        span: tok.span,
                    });
                }
            }
        }
    }

    Ok(body)
}

fn parse_state_decl(cursor: &mut TokenCursor) -> Result<StateDecl, MachineParseError> {
    let start_span = cursor.expect_kw("state")?;
    let (ident, ident_span) = cursor.expect_ident()?;
    let span = Span { start: start_span.start, end: ident_span.end };
    Ok(StateDecl { ident, span })
}

fn parse_transition_decl(cursor: &mut TokenCursor) -> Result<TransitionDecl, MachineParseError> {
    let start_span = cursor.expect_kw("transition")?;
    let (ident, ident_span) = cursor.expect_ident()?;
    let _ = ident_span;

    let params = if let Some(tok) = cursor.peek() {
        if tok.kind == TokKind::LParen {
            parse_params(cursor)?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    cursor.expect_punct(TokKind::LBrace)?;

    let mut from: Option<String> = None;
    if cursor.peek_is_kw("from") {
        cursor.expect_kw("from")?;
        cursor.expect_punct(TokKind::Colon)?;
        let (f, _) = cursor.expect_ident()?;
        from = Some(f);
    }

    let mut to: Option<String> = None;
    if cursor.peek_is_kw("to") {
        cursor.expect_kw("to")?;
        cursor.expect_punct(TokKind::Colon)?;
        let (t, _) = cursor.expect_ident()?;
        to = Some(t);
    }

    let mut guards = Vec::new();
    while cursor.peek_is_kw("guard") {
        let guard = parse_guard_clause(cursor)?;
        guards.push(guard);
    }

    let mut effects = Vec::new();
    if cursor.peek_is_kw("effect") {
        cursor.expect_kw("effect")?;
        cursor.expect_punct(TokKind::Colon)?;
        effects = parse_effect_list(cursor)?;
    }

    let has_content = !guards.is_empty() || (from.is_some() && to.is_some()) || !effects.is_empty();
    if !has_content {
        return Err(MachineParseError {
            message: "transition must have at least one of: guard clause, from+to pair, or effect list".to_string(),
            span: cursor.peek_span(),
        });
    }

    let close_span = cursor.expect_punct(TokKind::RBrace)?;
    let span = Span { start: start_span.start, end: close_span.end };

    Ok(TransitionDecl {
        ident,
        params,
        from,
        to,
        guards,
        effects,
        span,
    })
}

fn parse_params(cursor: &mut TokenCursor) -> Result<Vec<String>, MachineParseError> {
    cursor.expect_punct(TokKind::LParen)?;
    let mut params = Vec::new();

    let (first, _) = cursor.expect_ident()?;
    params.push(first);

    loop {
        match cursor.peek() {
            Some(tok) if tok.kind == TokKind::Comma => {
                cursor.advance();
                let (p, _) = cursor.expect_ident()?;
                params.push(p);
            }
            _ => break,
        }
    }

    cursor.expect_punct(TokKind::RParen)?;
    Ok(params)
}

fn parse_guard_clause(cursor: &mut TokenCursor) -> Result<GuardClause, MachineParseError> {
    let guard_span = cursor.expect_kw("guard")?;
    cursor.expect_punct(TokKind::Colon)?;

    let negated = if cursor.peek_is_kw("not") {
        cursor.expect_kw("not")?;
        true
    } else {
        false
    };

    let exists = parse_exists_expr(cursor)?;
    let span = Span { start: guard_span.start, end: exists.span.end };

    Ok(GuardClause { negated, exists, span })
}

fn parse_exists_expr(cursor: &mut TokenCursor) -> Result<ExistsExpr, MachineParseError> {
    let start_span = cursor.expect_kw("EXISTS")?;
    cursor.expect_punct(TokKind::LParen)?;
    let pattern = parse_pattern(cursor)?;
    let close_span = cursor.expect_punct(TokKind::RParen)?;
    let span = Span { start: start_span.start, end: close_span.end };

    Ok(ExistsExpr { pattern, span })
}

fn parse_pattern(cursor: &mut TokenCursor) -> Result<Pattern, MachineParseError> {
    let anchor = parse_pattern_term(cursor)?;

    let mut hops = Vec::new();
    while matches!(cursor.peek(), Some(Tok { kind: TokKind::Ident(_), .. })) {
        let hop = parse_pattern_hop(cursor)?;
        hops.push(hop);
    }

    if hops.is_empty() {
        return Err(MachineParseError {
            message: "pattern must have at least one hop".to_string(),
            span: cursor.peek_span(),
        });
    }

    Ok(Pattern { anchor, hops })
}

fn parse_pattern_hop(cursor: &mut TokenCursor) -> Result<PatternHop, MachineParseError> {
    // The predicate is an ident, but reserved words (e.g. "state") are
    // valid predicate names -- so this bypasses `expect_ident`'s
    // reserved-word rejection and checks `TokKind::Ident` directly.
    let eof = cursor.eof_span();
    let pred_tok = cursor.advance().ok_or_else(|| MachineParseError {
        message: "expected predicate identifier".to_string(),
        span: eof,
    })?;

    let predicate = match &pred_tok.kind {
        TokKind::Ident(s) => s.clone(),
        _ => {
            return Err(MachineParseError {
                message: "expected identifier for predicate".to_string(),
                span: pred_tok.span,
            })
        }
    };

    let term = parse_pattern_term(cursor)?;
    Ok(PatternHop { predicate, term })
}

fn parse_pattern_term(cursor: &mut TokenCursor) -> Result<PatternTerm, MachineParseError> {
    match cursor.peek() {
        Some(tok) => match &tok.kind {
            TokKind::Ident(s) if s == "self" => {
                cursor.advance();
                Ok(PatternTerm::SelfRef)
            }
            TokKind::Dollar => {
                cursor.advance();
                let ident = expect_any_ident(cursor, "expected identifier after '$'")?;
                Ok(PatternTerm::Param(ident))
            }
            TokKind::Question => {
                cursor.advance();
                let ident = expect_any_ident(cursor, "expected identifier after '?'")?;
                Ok(PatternTerm::Var(ident))
            }
            TokKind::Ident(_) => parse_node_ref(cursor),
            _ => Err(MachineParseError {
                message: "expected pattern term (self, $param, ?var, or node reference)".to_string(),
                span: tok.span,
            }),
        },
        None => Err(MachineParseError {
            message: "unexpected end of input, expected pattern term".to_string(),
            span: cursor.eof_span(),
        }),
    }
}

/// Consumes the next token as a plain identifier, bypassing
/// `TokenCursor::expect_ident`'s `RESERVED`-word rejection -- `$param`/
/// `?var` names (like node_ref segments and hop predicates) aren't
/// declaring anything in this machine's own closed vocabulary
/// (states/transitions), so a variable happening to be named e.g.
/// `guard` (colliding with the `guard:` keyword) is not ambiguous here
/// and must not be rejected.
fn expect_any_ident(cursor: &mut TokenCursor, message: &str) -> Result<String, MachineParseError> {
    let eof = cursor.eof_span();
    let tok = cursor.advance().ok_or_else(|| MachineParseError {
        message: message.to_string(),
        span: eof,
    })?;
    match &tok.kind {
        TokKind::Ident(s) => Ok(s.clone()),
        _ => Err(MachineParseError {
            message: message.to_string(),
            span: tok.span,
        }),
    }
}

fn parse_node_ref(cursor: &mut TokenCursor) -> Result<PatternTerm, MachineParseError> {
    let mut parts = Vec::new();

    let eof = cursor.eof_span();
    let first_tok = cursor.advance().ok_or_else(|| MachineParseError {
        message: "expected identifier for node reference".to_string(),
        span: eof,
    })?;

    match &first_tok.kind {
        TokKind::Ident(s) => parts.push(s.clone()),
        _ => {
            return Err(MachineParseError {
                message: "expected identifier".to_string(),
                span: first_tok.span,
            })
        }
    }

    loop {
        match cursor.peek() {
            Some(tok) if tok.kind == TokKind::Slash => {
                cursor.advance();
                let eof = cursor.eof_span();
                let next_tok = cursor.advance().ok_or_else(|| MachineParseError {
                    message: "expected identifier after '/'".to_string(),
                    span: eof,
                })?;
                match &next_tok.kind {
                    TokKind::Ident(s) => {
                        parts.push(s.clone());
                    }
                    _ => {
                        return Err(MachineParseError {
                            message: "expected identifier after '/'".to_string(),
                            span: next_tok.span,
                        })
                    }
                }
            }
            _ => break,
        }
    }

    let node_text = parts.join("/");
    Ok(PatternTerm::Node(node_text))
}

fn parse_effect_list(cursor: &mut TokenCursor) -> Result<Vec<Effect>, MachineParseError> {
    let mut effects = Vec::new();

    let first = parse_effect(cursor)?;
    effects.push(first);

    loop {
        match cursor.peek() {
            Some(tok) if tok.kind == TokKind::Comma => {
                cursor.advance();
                let eff = parse_effect(cursor)?;
                effects.push(eff);
            }
            _ => break,
        }
    }

    Ok(effects)
}

fn parse_effect(cursor: &mut TokenCursor) -> Result<Effect, MachineParseError> {
    match cursor.peek() {
        Some(tok) => {
            if tok.kind == TokKind::Ident("retract".to_string()) {
                cursor.advance();
                let (ident, _) = cursor.expect_ident()?;
                Ok(Effect::Retract(ident))
            } else if tok.kind == TokKind::Ident("assert".to_string()) {
                cursor.advance();
                let (ident, _) = cursor.expect_ident()?;
                Ok(Effect::Assert(ident))
            } else {
                Err(MachineParseError {
                    message: "expected 'retract' or 'assert'".to_string(),
                    span: tok.span,
                })
            }
        }
        None => Err(MachineParseError {
            message: "unexpected end of input, expected 'retract' or 'assert'".to_string(),
            span: cursor.eof_span(),
        }),
    }
}

/// Desugars `decl.from`/`decl.to` into the full guard/effect lists per
/// `MACHINE_SPEC.md`'s "Firing a transition" steps 1-2: if `from` is
/// present, prepend an implicit non-negated guard
/// `GuardClause { negated: false, exists: EXISTS(self, "state", from) }`
/// (using `decl.span` for that guard's own span, since it has no literal
/// source location of its own); if `to` is present (and `from` is also
/// present), append the implicit effects `[Retract(from), Assert(to)]`.
/// Returns the full resolved `(guards, effects)`, author-written entries
/// first, sugar-derived entries appended in the order described above.
pub fn resolve_transition(decl: &TransitionDecl) -> (Vec<GuardClause>, Vec<Effect>) {
    let mut guards = decl.guards.clone();

    if let Some(ref from_value) = decl.from {
        let implicit_guard = GuardClause {
            negated: false,
            exists: ExistsExpr {
                pattern: Pattern {
                    anchor: PatternTerm::SelfRef,
                    hops: vec![PatternHop {
                        predicate: "state".to_string(),
                        term: PatternTerm::Node(from_value.clone()),
                    }],
                },
                span: decl.span,
            },
            span: decl.span,
        };
        guards.insert(0, implicit_guard);
    }

    let mut effects = decl.effects.clone();

    if let (Some(ref from_value), Some(ref to_value)) = (&decl.from, &decl.to) {
        effects.push(Effect::Retract(from_value.clone()));
        effects.push(Effect::Assert(to_value.clone()));
    }

    (guards, effects)
}

/// Runtime bindings available while evaluating one transition firing's
/// guards, per `MACHINE_SPEC.md`'s "Evaluating EXISTS": the machine's
/// own node, and whatever `$param` values the commit firing the
/// transition supplied.
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    pub self_node: String,
    pub params: std::collections::HashMap<String, String>,
}

/// Evaluates one `EXISTS(pattern)` against `world`, per `MACHINE_SPEC.md`'s
/// "Evaluating EXISTS". Never panics.
///
/// Datalog-backed as of the cutover that added `crate::datalog_guard`
/// (see that module's own doc comment for the full design and the
/// equivalence tests it was proven against before this delegation
/// replaced the hand-rolled walker that used to live here). Kept as a
/// stable, named function -- not inlined at call sites -- since this
/// crate's own test suite (`tests/machine_eval_examples.rs` and others)
/// calls `eval_exists` directly by name.
pub fn eval_exists(pattern: &Pattern, ctx: &EvalContext, world: &crate::interpret::Materialized) -> bool {
    crate::datalog_guard::eval_exists(pattern, ctx, world)
}

/// Evaluates one `GuardClause` (its `EXISTS` result, XORed with
/// `negated`) against `world`.
pub fn eval_guard(guard: &GuardClause, ctx: &EvalContext, world: &crate::interpret::Materialized) -> bool {
    eval_exists(&guard.exists.pattern, ctx, world) != guard.negated
}

/// Evaluates a full guard list (a `TransitionDecl`'s resolved `guards`,
/// after `resolve_transition`'s sugar) against `world` -- plain
/// conjunction, per "Firing a transition": the transition may fire iff
/// every guard holds.
pub fn eval_guards(guards: &[GuardClause], ctx: &EvalContext, world: &crate::interpret::Materialized) -> bool {
    guards.iter().all(|guard| eval_guard(guard, ctx, world))
}

/// Parses every `machine_stmt` in `doc`, keyed by the machine's own
/// node, joined the same way `crate::lower::lower_reference` joins a
/// `NodeRef`'s segments (`stmt.node.segments.join("/")`). Stops at the
/// first malformed machine body, in document order.
pub fn parse_all_machines(
    doc: &crate::ast::Document,
) -> Result<std::collections::HashMap<String, MachineBody>, (String, MachineParseError)> {
    let mut map = std::collections::HashMap::new();
    for item in &doc.items {
        if let crate::ast::TopLevelItem::Machine(stmt) = item {
            let key = stmt.node.segments.join("/");
            match parse_machine_body(&stmt.body) {
                Ok(machine_body) => {
                    map.insert(key, machine_body);
                }
                Err(e) => {
                    return Err((key, e));
                }
            }
        }
    }
    Ok(map)
}

/// Whether `ident`'s transition may fire right now, given `ctx` and
/// `world`: resolves the transition (`from`/`to` sugar included) and
/// evaluates its guard list. `None` if no transition with that ident is
/// declared in `body` -- distinct from `Some(false)` ("declared, but
/// blocked").
pub fn may_fire(
    body: &MachineBody,
    ident: &str,
    ctx: &EvalContext,
    world: &crate::interpret::Materialized,
) -> Option<bool> {
    let decl = body.transitions.iter().find(|t| t.ident == ident)?;
    let (guards, _) = resolve_transition(decl);
    Some(eval_guards(&guards, ctx, world))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FiresTransitionError {
    /// No transition named `ident` is declared on this machine.
    UnknownTransition,
    /// The transition's guards did not hold against `world_before` --
    /// this commit was never entitled to fire, regardless of what it
    /// actually asserts.
    GuardNotSatisfied,
    /// The guard held, but the candidate's own `consumes`/`produces`
    /// don't match the transition's resolved effects: `missing` lists
    /// every resolved effect the candidate never actually delivered.
    EffectMismatch { missing: Vec<Effect> },
}

/// Whether a candidate commit's own `consumes`/`produces` actually
/// deliver exactly the effects `ident`'s resolved transition requires,
/// evaluated against `world_before` (the materialized state
/// immediately prior to this candidate committing) -- the "did this
/// commit fire it correctly" half `MACHINE_SPEC.md`'s own "Wiring into
/// the toolchain" section left deferred pending issue #70's
/// retraction-aware materialization, now real.
///
/// Distinct from `may_fire`: `may_fire` asks "is this transition
/// currently permitted to fire" (a guard question against the current
/// world). This asks "does THIS SPECIFIC commit's content match what
/// firing `ident` actually requires" (an effects-matching question
/// against a candidate commit) -- a resolver needs both, in order:
/// first confirm the guard held immediately before the candidate
/// (`may_fire` against `world_before`), then confirm the candidate's
/// own triples are exactly the resolved effects, not something else
/// asserted under the same transition name.
///
/// Every resolved effect currently checks predicate `"state"`
/// specifically, on `ctx.self_node` -- not a shortcut: both `from`/`to`
/// sugar-derived effects AND author-written explicit `retract`/`assert`
/// effects share one value-only grammar (`parse_effect`), always
/// implicitly `(self, "state", <value>)`. If that grammar ever grows a
/// full-triple explicit-effect form, this hardcoding stops being
/// implied by the grammar and needs revisiting alongside it.
///
/// A `ConsumeRef::Strong` (whole-commit reference) never satisfies a
/// `Retract` effect here, by deliberate choice, not an oversight:
/// `interpret::apply_consume` already treats a `Strong` consume as
/// retracting every `(subject, predicate)` its target commit produced,
/// so a `Strong` reference genuinely *could* deliver a given `Retract`
/// effect -- but confirming that from inside this function would mean
/// resolving the referenced commit's own content, which `world_before`
/// (a plain `Materialized` fold, not an `IdentifiedCommit` history) has
/// no way to look up. Accepting a `Strong` reference unconditionally
/// (assuming it always retracts whatever's needed) would be the wrong
/// kind of wrong: a commit could reference an unrelated `Strong` target
/// and this function would wave the effect through unverified. Failing
/// closed (reporting `EffectMismatch` rather than guessing `Ok`) matches
/// this crate's own posture on unverifiable claims elsewhere
/// (`cross_repo_commit_valid`'s fail-closed stance, not
/// `commit_valid_despite_dangling_factref`'s fail-open one -- a
/// different category of problem, verifying a *positive* claim rather
/// than tolerating a *dangling* one). Resolving a `Strong` reference's
/// real content, if this ever needs to stop being conservative here,
/// is real follow-up work needing the caller to supply the underlying
/// commit history, not a fix confined to this function's current
/// signature.
pub fn commit_fires_transition(
    body: &MachineBody,
    ident: &str,
    ctx: &EvalContext,
    world_before: &crate::interpret::Materialized,
    candidate: &crate::lower::LoweredCommit,
) -> Result<(), FiresTransitionError> {
    let decl = body
        .transitions
        .iter()
        .find(|t| t.ident == ident)
        .ok_or(FiresTransitionError::UnknownTransition)?;

    let (guards, effects) = resolve_transition(decl);

    if !eval_guards(&guards, ctx, world_before) {
        return Err(FiresTransitionError::GuardNotSatisfied);
    }

    let mut missing: Vec<Effect> = Vec::new();

    for effect in &effects {
        let delivered = match effect {
            Effect::Assert(value) => candidate.produces.iter().any(|t| {
                t.subject == ctx.self_node
                    && t.predicate == "state"
                    && t.object == crate::lower::TripleValue::Node(value.clone())
            }),
            Effect::Retract(value) => candidate.consumes.iter().any(|cr| match cr {
                crate::lower::ConsumeRef::Fact(fr) => {
                    if fr.subject != ctx.self_node || fr.predicate != "state" {
                        return false;
                    }
                    let has_object = fr.object.is_some();
                    let object_equal =
                        fr.object.as_ref() == Some(&crate::lower::TripleValue::Node(value.clone()));
                    crate::resolver::factref_matches(has_object, object_equal)
                }
                crate::lower::ConsumeRef::Strong(_) => false,
            }),
        };

        if !delivered {
            missing.push(effect.clone());
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(FiresTransitionError::EffectMismatch { missing })
    }
}
