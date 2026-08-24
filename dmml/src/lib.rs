//! DMML (Desiring-Machine Markup Language) reference parser.
//!
//! Implements the EBNF grammar from `SPEC.md` section 10 ("Formal grammar
//! (EBNF)") as a hand-written recursive-descent parser -- no
//! parser-generator dependency, per that section's own justification
//! (error-message quality, minimal build-pipeline surface area).
//!
//! `parse` never panics on any input, including malformed or adversarial
//! byte sequences: every rejection is a `ParseError`, not a panic. This is
//! the property the fuzz target in `fuzz/` checks continuously.

pub mod ast;
pub mod error;
pub mod from_json;
pub mod genesis;
pub mod identity;
pub mod interpret;
mod lexer;
pub mod lower;
pub mod machine;
mod parser;
pub mod resolver;
pub mod validate;
pub mod view;

pub use ast::Document;
pub use error::ParseError;

/// Parses `input` as a DMML document. Never panics.
pub fn parse(input: &str) -> Result<Document, ParseError> {
    parser::parse_document(input)
}
