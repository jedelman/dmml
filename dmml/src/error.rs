use crate::ast::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        ParseError {
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (at bytes {}..{})",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ParseError {}
