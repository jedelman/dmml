use crate::ast::*;
use crate::error::ParseError;
use crate::lexer::{tokenize, Token, TokenKind};

struct Parser<'a> {
    input: &'a str,
    tokens: Vec<Token>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &Token {
        // `tokens` always ends with an Eof token and `pos` only ever
        // advances past a non-Eof token, so this is always in range.
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn peek_ahead(&self, n: usize) -> Option<&TokenKind> {
        self.tokens.get(self.pos + n).map(|t| &t.kind)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token {
            kind: TokenKind::Eof,
            span: self.eof_span(),
        });
        if !matches!(tok.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    fn eof_span(&self) -> Span {
        Span {
            start: self.input.len(),
            end: self.input.len(),
        }
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError::new(message.into(), self.peek().span)
    }

    fn describe(kind: &TokenKind) -> String {
        match kind {
            TokenKind::Ident(s) => format!("identifier `{s}`"),
            TokenKind::Number(s) => format!("number `{s}`"),
            TokenKind::Str(s) => format!("string {s:?}"),
            TokenKind::AtUri(s) => format!("at-uri `{s}`"),
            TokenKind::OpaqueBlock(_) => "`{ ... }`".to_string(),
            TokenKind::LBrace => "`{`".to_string(),
            TokenKind::RBrace => "`}`".to_string(),
            TokenKind::LParen => "`(`".to_string(),
            TokenKind::RParen => "`)`".to_string(),
            TokenKind::Colon => "`:`".to_string(),
            TokenKind::Slash => "`/`".to_string(),
            TokenKind::Dot => "`.`".to_string(),
            TokenKind::Eof => "end of input".to_string(),
        }
    }

    fn peek_ident_text(&self) -> Option<&str> {
        match &self.peek().kind {
            TokenKind::Ident(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn expect_ident(&mut self, text: &str) -> Result<Span, ParseError> {
        match &self.peek().kind {
            TokenKind::Ident(s) if s == text => {
                let span = self.peek().span;
                self.advance();
                Ok(span)
            }
            other => Err(ParseError::new(
                format!("expected `{text}`, found {}", Self::describe(other)),
                self.peek().span,
            )),
        }
    }

    fn expect_any_ident(&mut self) -> Result<(String, Span), ParseError> {
        match &self.peek().kind {
            TokenKind::Ident(s) => {
                let s = s.clone();
                let span = self.peek().span;
                self.advance();
                Ok((s, span))
            }
            other => Err(ParseError::new(
                format!("expected an identifier, found {}", Self::describe(other)),
                self.peek().span,
            )),
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Span, ParseError> {
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(&kind) {
            let span = self.peek().span;
            self.advance();
            Ok(span)
        } else {
            let found = Self::describe(&self.peek().kind);
            Err(ParseError::new(
                format!("expected {}, found {found}", Self::describe(&kind)),
                self.peek().span,
            ))
        }
    }

    fn parse_document(&mut self) -> Result<Document, ParseError> {
        let mut items = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Eof) {
            items.push(self.parse_top_level_item()?);
        }
        Ok(Document { items })
    }

    fn parse_top_level_item(&mut self) -> Result<TopLevelItem, ParseError> {
        match self.peek_ident_text() {
            Some("commit") => Ok(TopLevelItem::Commit(self.parse_commit_stmt()?)),
            Some("reference") => Ok(TopLevelItem::Reference(self.parse_reference_stmt()?)),
            Some("machine") => Ok(TopLevelItem::Machine(self.parse_machine_stmt()?)),
            _ => Err(self.error(format!(
                "expected `commit`, `reference`, or `machine`, found {}",
                Self::describe(&self.peek().kind)
            ))),
        }
    }

    fn parse_commit_stmt(&mut self) -> Result<CommitStmt, ParseError> {
        let start = self.expect_ident("commit")?.start;
        let (predicate_verb, _) = self.expect_any_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut items = Vec::new();
        loop {
            match self.peek().kind {
                TokenKind::RBrace => break,
                TokenKind::Eof => {
                    return Err(self.error("unexpected end of input inside `commit { ... }`"))
                }
                _ => items.push(self.parse_commit_item()?),
            }
        }
        let end = self.expect(TokenKind::RBrace)?.end;

        Ok(CommitStmt {
            predicate_verb,
            items,
            span: Span { start, end },
        })
    }

    fn parse_commit_item(&mut self) -> Result<CommitItem, ParseError> {
        match self.peek_ident_text() {
            Some("declare") => Ok(CommitItem::Declare(self.parse_declare_stmt()?)),
            Some("produces") => Ok(CommitItem::Produces(self.parse_produces_block()?)),
            Some("consumes") => Ok(CommitItem::Consumes(self.parse_consumes_block()?)),
            Some("via") => {
                self.advance();
                Ok(CommitItem::Via(self.parse_strong_ref()?))
            }
            Some("respondsTo") => {
                self.advance();
                Ok(CommitItem::RespondsTo(self.parse_strong_ref()?))
            }
            Some(_) => Ok(CommitItem::Fact(self.parse_fact_stmt()?)),
            None => Err(self.error(format!(
                "expected a commit item (`declare`, `produces`, `consumes`, `via`, \
                 `respondsTo`, or a fact), found {}",
                Self::describe(&self.peek().kind)
            ))),
        }
    }

    fn parse_declare_stmt(&mut self) -> Result<DeclareStmt, ParseError> {
        let start = self.expect_ident("declare")?.start;
        let (kind_text, kind_span) = self.expect_any_ident()?;
        let kind = match kind_text.as_str() {
            "relation" => DeclKind::Relation,
            "attribute" => DeclKind::Attribute,
            other => {
                return Err(ParseError::new(
                    format!("expected `relation` or `attribute`, found identifier `{other}`"),
                    kind_span,
                ))
            }
        };
        let (ident, ident_span) = self.expect_any_ident()?;
        Ok(DeclareStmt {
            kind,
            ident,
            span: Span {
                start,
                end: ident_span.end,
            },
        })
    }

    fn parse_produces_block(&mut self) -> Result<ProducesBlock, ParseError> {
        let start = self.expect_ident("produces")?.start;
        self.expect(TokenKind::LBrace)?;
        let mut facts = Vec::new();
        loop {
            match self.peek().kind {
                TokenKind::RBrace => break,
                TokenKind::Eof => {
                    return Err(self.error("unexpected end of input inside `produces { ... }`"))
                }
                _ => facts.push(self.parse_fact_stmt()?),
            }
        }
        let end = self.expect(TokenKind::RBrace)?.end;
        Ok(ProducesBlock {
            facts,
            span: Span { start, end },
        })
    }

    fn parse_fact_stmt(&mut self) -> Result<FactStmt, ParseError> {
        let subject = self.parse_node_ref()?;
        let predicate = self.parse_predicate_ref()?;
        let value = self.parse_value()?;
        let end = match &value {
            Value::Node(n) => n.span.end,
            Value::Literal(_) => self.tokens[self.pos.saturating_sub(1)].span.end,
        };
        Ok(FactStmt {
            span: Span {
                start: subject.span.start,
                end,
            },
            subject,
            predicate,
            value,
        })
    }

    fn parse_predicate_ref(&mut self) -> Result<PredicateRef, ParseError> {
        let (text, _) = self.expect_any_ident()?;
        if text == "a" {
            Ok(PredicateRef::RdfType)
        } else {
            Ok(PredicateRef::Ident(text))
        }
    }

    /// `value = node_ref | literal`. A bare `Number` token is a numeric
    /// literal unless it's immediately followed by a `.` or `/` token, in
    /// which case it's actually the leading segment of a node_ref (e.g.
    /// `42.reach`, `42/foo`) -- one token of lookahead disambiguates
    /// without any raw-text scanning.
    fn parse_value(&mut self) -> Result<Value, ParseError> {
        match &self.peek().kind {
            TokenKind::Str(_) => {
                let TokenKind::Str(s) = self.advance().kind else {
                    unreachable!()
                };
                Ok(Value::Literal(Literal::String(s)))
            }
            TokenKind::Ident(s) if s == "true" => {
                self.advance();
                Ok(Value::Literal(Literal::Boolean(true)))
            }
            TokenKind::Ident(s) if s == "false" => {
                self.advance();
                Ok(Value::Literal(Literal::Boolean(false)))
            }
            TokenKind::Number(_) => {
                let pathish = matches!(
                    self.peek_ahead(1),
                    Some(TokenKind::Dot) | Some(TokenKind::Slash)
                );
                if pathish {
                    Ok(Value::Node(self.parse_node_ref()?))
                } else {
                    let TokenKind::Number(s) = self.advance().kind else {
                        unreachable!()
                    };
                    Ok(Value::Literal(Literal::Number(s)))
                }
            }
            TokenKind::Ident(_) => Ok(Value::Node(self.parse_node_ref()?)),
            other => Err(ParseError::new(
                format!(
                    "expected a value (node reference, number, boolean, or string), found {}",
                    Self::describe(other)
                ),
                self.peek().span,
            )),
        }
    }

    /// One `ident , { "/" , ident }` production, where each dot-joined
    /// piece (`Ident`/`Number` tokens separated by `Dot` tokens, e.g.
    /// `42.reach`) collapses into a single segment string -- this is a
    /// grammar-level widening past the EBNF's plain `ident` beyond what's
    /// written in `SPEC.md` today: real DMML content in that same
    /// document (`room/42`, `room/42.reach`) needs digit-leading and
    /// dotted segments, which a strict `letter , {letter|digit|"_"}`
    /// `ident` cannot produce. Surfaced by actually building this parser
    /// against the spec's own examples, not assumed going in.
    fn parse_node_ref(&mut self) -> Result<NodeRef, ParseError> {
        let (first, first_span) = self.parse_segment()?;
        let mut segments = vec![first];
        let mut end = first_span.end;
        while matches!(self.peek().kind, TokenKind::Slash) {
            self.advance();
            let (seg, seg_span) = self.parse_segment()?;
            segments.push(seg);
            end = seg_span.end;
        }
        Ok(NodeRef {
            segments,
            span: Span {
                start: first_span.start,
                end,
            },
        })
    }

    fn parse_segment(&mut self) -> Result<(String, Span), ParseError> {
        let (mut text, first_span) = self.parse_segment_piece()?;
        let mut end = first_span.end;
        while matches!(self.peek().kind, TokenKind::Dot) {
            self.advance();
            let (piece, piece_span) = self.parse_segment_piece()?;
            text.push('.');
            text.push_str(&piece);
            end = piece_span.end;
        }
        Ok((
            text,
            Span {
                start: first_span.start,
                end,
            },
        ))
    }

    fn parse_segment_piece(&mut self) -> Result<(String, Span), ParseError> {
        match &self.peek().kind {
            TokenKind::Ident(s) => {
                let s = s.clone();
                let span = self.peek().span;
                self.advance();
                Ok((s, span))
            }
            TokenKind::Number(s) => {
                let s = s.clone();
                let span = self.peek().span;
                self.advance();
                Ok((s, span))
            }
            other => Err(ParseError::new(
                format!(
                    "expected a node reference segment, found {}",
                    Self::describe(other)
                ),
                self.peek().span,
            )),
        }
    }

    fn parse_consumes_block(&mut self) -> Result<ConsumesBlock, ParseError> {
        let start = self.expect_ident("consumes")?.start;
        self.expect(TokenKind::LBrace)?;
        let mut entries = Vec::new();
        loop {
            match self.peek().kind {
                TokenKind::RBrace => break,
                TokenKind::Eof => {
                    return Err(self.error("unexpected end of input inside `consumes { ... }`"))
                }
                _ => entries.push(self.parse_consume_entry()?),
            }
        }
        let end = self.expect(TokenKind::RBrace)?.end;
        Ok(ConsumesBlock {
            entries,
            span: Span { start, end },
        })
    }

    fn parse_consume_entry(&mut self) -> Result<ConsumeEntry, ParseError> {
        match self.peek_ident_text() {
            Some("strong") => {
                self.advance();
                Ok(ConsumeEntry::Strong(self.parse_strong_ref()?))
            }
            Some("fact") => Ok(ConsumeEntry::Fact(self.parse_fact_consume()?)),
            _ => Err(self.error(format!(
                "expected `strong` or `fact`, found {}",
                Self::describe(&self.peek().kind)
            ))),
        }
    }

    fn parse_fact_consume(&mut self) -> Result<FactConsume, ParseError> {
        let start = self.expect_ident("fact")?.start;
        let commit = self.parse_strong_ref()?;
        self.expect(TokenKind::LBrace)?;
        self.expect_ident("subject")?;
        self.expect(TokenKind::Colon)?;
        let subject = self.parse_node_ref()?;
        self.expect_ident("predicate")?;
        self.expect(TokenKind::Colon)?;
        let (predicate, _) = self.expect_any_ident()?;
        let mut object = None;
        if let Some("object") = self.peek_ident_text() {
            self.advance();
            self.expect(TokenKind::Colon)?;
            object = Some(self.parse_value()?);
        }
        let end = self.expect(TokenKind::RBrace)?.end;
        Ok(FactConsume {
            commit,
            subject,
            predicate,
            object,
            span: Span { start, end },
        })
    }

    fn parse_strong_ref(&mut self) -> Result<StrongRef, ParseError> {
        let (raw, uri_span) = match &self.peek().kind {
            TokenKind::AtUri(s) => (s.clone(), self.peek().span),
            other => {
                return Err(ParseError::new(
                    format!("expected an at:// uri, found {}", Self::describe(other)),
                    self.peek().span,
                ))
            }
        };
        self.advance();

        let uri = parse_at_uri(&raw, uri_span)?;

        self.expect(TokenKind::LParen)?;
        self.expect_ident("cid")?;
        self.expect(TokenKind::Colon)?;

        // The lexer has no dedicated CID token; scan raw source text
        // directly for a run of non-whitespace, non-')' characters
        // (skipping the whitespace that ordinarily separates `:` from the
        // cid text first), then resynchronize the token cursor past
        // whatever tokens that raw region consumed.
        let mut cid_start = self.last_consumed_end();
        while matches!(self.input[cid_start..].chars().next(), Some(c) if c.is_whitespace()) {
            cid_start += self.input[cid_start..].chars().next().unwrap().len_utf8();
        }

        let mut idx = cid_start;
        for (i, c) in self.input[cid_start..].char_indices() {
            let abs = cid_start + i;
            if c.is_whitespace() || c == ')' {
                idx = abs;
                break;
            }
            idx = abs + c.len_utf8();
        }
        if idx == cid_start {
            return Err(ParseError::new(
                "expected a cid after `cid:`".to_string(),
                Span {
                    start: cid_start,
                    end: cid_start,
                },
            ));
        }
        let cid = self.input[cid_start..idx].to_string();
        self.resync_past(idx);

        let end = self.expect(TokenKind::RParen)?.end;

        Ok(StrongRef {
            uri,
            cid,
            span: Span {
                start: uri_span.start,
                end,
            },
        })
    }

    /// Byte offset one past the token most recently consumed by `advance`.
    fn last_consumed_end(&self) -> usize {
        if self.pos == 0 {
            0
        } else {
            self.tokens[self.pos - 1].span.end
        }
    }

    /// Advances `self.pos` past every already-lexed token whose span ends
    /// at or before `byte_offset`, discarding them as already consumed by
    /// a raw-text scan (used after manually scanning the cid region).
    fn resync_past(&mut self, byte_offset: usize) {
        while self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            if matches!(tok.kind, TokenKind::Eof) {
                break;
            }
            if tok.span.end <= byte_offset {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse_reference_stmt(&mut self) -> Result<ReferenceStmt, ParseError> {
        let start = self.expect_ident("reference")?.start;
        let target = self.parse_strong_ref()?;
        let mut end = target.span.end;
        let mut as_name = None;
        if let Some("as") = self.peek_ident_text() {
            self.advance();
            let name = self.parse_node_ref()?;
            end = name.span.end;
            as_name = Some(name);
        }
        Ok(ReferenceStmt {
            target,
            as_name,
            span: Span { start, end },
        })
    }

    fn parse_machine_stmt(&mut self) -> Result<MachineStmt, ParseError> {
        let start = self.expect_ident("machine")?.start;
        let node = self.parse_node_ref()?;

        match &self.peek().kind {
            TokenKind::OpaqueBlock(_) => {}
            other => {
                return Err(ParseError::new(
                    format!("expected `{{`, found {}", Self::describe(other)),
                    self.peek().span,
                ))
            }
        }

        let tok = self.advance();
        let TokenKind::OpaqueBlock(raw) = tok.kind else {
            unreachable!("matched above")
        };
        // `raw` covers the outer braces inclusive (per the lexer's
        // OpaqueBlock contract); MachineStmt::body is strictly between
        // them, matching scan_balanced_braces' own contract.
        let body = raw
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .unwrap_or(&raw)
            .to_string();

        Ok(MachineStmt {
            node,
            body,
            span: Span {
                start,
                end: tok.span.end,
            },
        })
    }
}

fn parse_at_uri(raw: &str, span: Span) -> Result<AtUri, ParseError> {
    let rest = raw.strip_prefix("at://").ok_or_else(|| {
        ParseError::new(
            format!("malformed at-uri `{raw}`: missing `at://` prefix"),
            span,
        )
    })?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) {
        return Err(ParseError::new(
            format!(
                "malformed at-uri `{raw}`: expected exactly did/nsid/rkey, found {} segment(s)",
                parts.len()
            ),
            span,
        ));
    }
    Ok(AtUri {
        raw: raw.to_string(),
        did: parts[0].to_string(),
        nsid: parts[1].to_string(),
        rkey: parts[2].to_string(),
    })
}

pub(crate) fn parse_document(input: &str) -> Result<Document, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser {
        input,
        tokens,
        pos: 0,
    };
    parser.parse_document()
}
