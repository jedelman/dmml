use crate::ast::Span;
use crate::error::ParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ident(String),
    Number(String),
    Str(String),
    AtUri(String),
    /// The entire text between (exclusive) a `machine <node_ref> { ... }`
    /// statement's outer braces, captured without being tokenized -- see
    /// `tokenize`'s `pending_machine` handling. Span covers the outer
    /// braces themselves (inclusive), matching `scan_balanced_braces`'s
    /// `end_idx`.
    OpaqueBlock(String),
    LBrace,
    RBrace,
    LParen,
    RParen,
    Colon,
    Slash,
    Dot,
    Eof,
}

/// A simple byte-offset cursor over `input`, re-slicing rather than
/// holding a persistent character iterator -- this makes arbitrary
/// seeking (needed to skip a `machine` block's opaque body) trivial and
/// panic-free, at the cost of re-scanning from `pos` on every `peek`.
/// Not performance-critical for a reference implementation.
struct Cursor<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str) -> Self {
        Cursor { input, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }
}

struct Lexer<'a> {
    cursor: Cursor<'a>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Lexer {
            cursor: Cursor::new(input),
        }
    }

    fn error(&self, message: String, start: usize) -> ParseError {
        ParseError::new(
            message,
            Span {
                start,
                end: self.cursor.pos,
            },
        )
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while matches!(self.cursor.peek(), Some(c) if c.is_whitespace()) {
                self.cursor.bump();
            }

            if self.cursor.peek() == Some('/') {
                let save = self.cursor.pos;
                self.cursor.bump();
                if self.cursor.peek() == Some('/') {
                    self.cursor.bump();
                    loop {
                        match self.cursor.peek() {
                            Some('\n') | None => break,
                            _ => {
                                self.cursor.bump();
                            }
                        }
                    }
                    continue;
                } else {
                    self.cursor.seek(save);
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn read_ident(&mut self, start: usize) -> Token {
        while matches!(self.cursor.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            self.cursor.bump();
        }
        let end = self.cursor.pos;
        Token {
            kind: TokenKind::Ident(self.cursor.input[start..end].to_string()),
            span: Span { start, end },
        }
    }

    fn read_number(&mut self, start: usize) -> Token {
        while matches!(self.cursor.peek(), Some(c) if c.is_ascii_digit()) {
            self.cursor.bump();
        }

        if self.cursor.peek() == Some('.') {
            let save = self.cursor.pos;
            self.cursor.bump();
            if matches!(self.cursor.peek(), Some(c) if c.is_ascii_digit()) {
                while matches!(self.cursor.peek(), Some(c) if c.is_ascii_digit()) {
                    self.cursor.bump();
                }
            } else {
                // Not a decimal point (no digit follows) -- it belongs to
                // whatever comes next (e.g. a dotted node-ref segment like
                // `42.reach`), not to this number. Rewind.
                self.cursor.seek(save);
            }
        }

        let end = self.cursor.pos;
        Token {
            kind: TokenKind::Number(self.cursor.input[start..end].to_string()),
            span: Span { start, end },
        }
    }

    fn read_string(&mut self, start: usize) -> Result<Token, ParseError> {
        // Opening '"' already consumed by the caller.
        let mut content = String::new();
        loop {
            match self.cursor.peek() {
                None => return Err(self.error("unterminated string literal".to_string(), start)),
                Some('\\') => {
                    self.cursor.bump();
                    match self.cursor.peek() {
                        Some('"') => {
                            content.push('"');
                            self.cursor.bump();
                        }
                        Some('\\') => {
                            content.push('\\');
                            self.cursor.bump();
                        }
                        Some(other) => {
                            content.push('\\');
                            content.push(other);
                            self.cursor.bump();
                        }
                        None => {
                            return Err(self.error(
                                "unterminated string literal after backslash".to_string(),
                                start,
                            ))
                        }
                    }
                }
                Some('"') => {
                    self.cursor.bump();
                    let end = self.cursor.pos;
                    return Ok(Token {
                        kind: TokenKind::Str(content),
                        span: Span { start, end },
                    });
                }
                Some(c) => {
                    content.push(c);
                    self.cursor.bump();
                }
            }
        }
    }

    /// If the input at `start` begins with the literal `at://`, consumes a
    /// maximal run of non-whitespace, non-`(` characters as an `AtUri`
    /// token. Otherwise consumes nothing and returns `None`.
    fn try_read_at_uri(&mut self, start: usize) -> Option<Token> {
        if !self.cursor.input[start..].starts_with("at://") {
            return None;
        }

        let save = self.cursor.pos;
        while matches!(self.cursor.peek(), Some(c) if !c.is_whitespace() && c != '(') {
            self.cursor.bump();
        }

        let end = self.cursor.pos;
        if end <= start + "at://".len() {
            self.cursor.seek(save);
            return None;
        }

        Some(Token {
            kind: TokenKind::AtUri(self.cursor.input[start..end].to_string()),
            span: Span { start, end },
        })
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_whitespace_and_comments();

        let start = self.cursor.pos;
        let c = match self.cursor.bump() {
            Some(c) => c,
            None => {
                return Ok(Token {
                    kind: TokenKind::Eof,
                    span: Span { start, end: start },
                })
            }
        };

        let simple = |kind: TokenKind, end: usize| Token {
            kind,
            span: Span { start, end },
        };

        match c {
            '{' => Ok(simple(TokenKind::LBrace, self.cursor.pos)),
            '}' => Ok(simple(TokenKind::RBrace, self.cursor.pos)),
            '(' => Ok(simple(TokenKind::LParen, self.cursor.pos)),
            ')' => Ok(simple(TokenKind::RParen, self.cursor.pos)),
            ':' => Ok(simple(TokenKind::Colon, self.cursor.pos)),
            '/' => Ok(simple(TokenKind::Slash, self.cursor.pos)),
            '.' => Ok(simple(TokenKind::Dot, self.cursor.pos)),
            '"' => self.read_string(start),
            '-' => {
                if matches!(self.cursor.peek(), Some(c2) if c2.is_ascii_digit()) {
                    Ok(self.read_number(start))
                } else {
                    Err(self.error(
                        format!("unexpected character '-' at position {start}"),
                        start,
                    ))
                }
            }
            c if c.is_ascii_digit() => Ok(self.read_number(start)),
            c if c.is_ascii_alphabetic() => {
                if let Some(tok) = self.try_read_at_uri(start) {
                    Ok(tok)
                } else {
                    Ok(self.read_ident(start))
                }
            }
            c => Err(self.error(
                format!("unexpected character {c:?} at position {start}"),
                start,
            )),
        }
    }
}

/// Tokenizes the entire document. `machine <node_ref> { ... }` bodies are
/// never lexed structurally: per `SPEC.md`'s grammar, a `machine` block's
/// interior is reserved-but-unspecified and may contain characters (a
/// literal comma, arbitrary punctuation) this lexer's general grammar
/// doesn't recognize at all -- lexing it as ordinary DMML would make any
/// such content a spurious lex error before the parser ever gets a chance
/// to treat it as opaque. Recognized structurally, at the token level:
/// once a depth-0 `Ident("machine")` token is emitted, the following
/// `Ident`/`Slash`/`Dot` tokens (its `node_ref`) are tokenized normally,
/// and the very next `{` at that point is treated as the start of an
/// opaque block rather than an ordinary `LBrace` -- its whole balanced
/// interior is captured raw via `scan_balanced_braces` and emitted as one
/// `OpaqueBlock` token, and tokenization resumes immediately after it.
pub fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    let mut depth: i32 = 0;
    let mut pending_machine = false;

    loop {
        lexer.skip_whitespace_and_comments();

        if pending_machine && depth == 0 && lexer.cursor.peek() == Some('{') {
            let open_idx = lexer.cursor.pos;
            let (_, end_idx) = scan_balanced_braces(input, open_idx)?;
            tokens.push(Token {
                kind: TokenKind::OpaqueBlock(input[open_idx..end_idx].to_string()),
                span: Span {
                    start: open_idx,
                    end: end_idx,
                },
            });
            lexer.cursor.seek(end_idx);
            pending_machine = false;
            continue;
        }

        let tok = lexer.next_token()?;

        match &tok.kind {
            TokenKind::Ident(s) if s == "machine" && depth == 0 => pending_machine = true,
            TokenKind::Ident(_) | TokenKind::Number(_) | TokenKind::Slash | TokenKind::Dot
                if pending_machine =>
            {
                // Still inside the machine statement's node_ref (which may
                // contain digit-leading or dotted segments, e.g.
                // `edge/12`) -- keep waiting for the opaque-block '{'.
            }
            TokenKind::LBrace => {
                depth += 1;
                pending_machine = false;
            }
            TokenKind::RBrace => {
                depth -= 1;
                pending_machine = false;
            }
            _ => pending_machine = false,
        }

        let is_eof = matches!(tok.kind, TokenKind::Eof);
        tokens.push(tok);
        if is_eof {
            break;
        }
    }

    Ok(tokens)
}

/// Starting at byte offset `open_brace_idx` in `input` (the index of an
/// opening '{'), scans forward tracking nested '{'/'}' depth until it
/// returns to 0. Returns (body, end_idx): body is the raw text strictly
/// between the outer braces (exclusive), end_idx is one past the matching
/// '}'. Never panics; unmatched braces are a ParseError.
pub fn scan_balanced_braces(
    input: &str,
    open_brace_idx: usize,
) -> Result<(String, usize), ParseError> {
    if !input.is_char_boundary(open_brace_idx) || !input[open_brace_idx..].starts_with('{') {
        return Err(ParseError::new(
            format!("expected '{{' at position {open_brace_idx}"),
            Span {
                start: open_brace_idx,
                end: open_brace_idx,
            },
        ));
    }

    let mut depth: u32 = 1;
    let body_start = open_brace_idx + 1;

    for (idx, c) in input[body_start..].char_indices() {
        let abs_idx = body_start + idx;
        match c {
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let body = input[body_start..abs_idx].to_string();
                    let end_idx = abs_idx + 1;
                    return Ok((body, end_idx));
                }
            }
            _ => {}
        }
    }

    Err(ParseError::new(
        "unmatched '{' in input".to_string(),
        Span {
            start: open_brace_idx,
            end: input.len(),
        },
    ))
}
