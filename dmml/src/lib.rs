//! DMML (Desiring-Machine Markup Language) ontology and reference
//! materializer.
//!
//! Authoring happens through JSON (`from_json`), deserialized directly
//! into `ast::Document`'s node types -- there is no text grammar and no
//! text parser. DMML used to be a hand-written source language with its
//! own recursive-descent parser (retired along with the lexer/parser
//! modules and the fuzz target that exercised them; see `from_json`'s own
//! doc comment for why: nothing hand-writes DMML source text, only
//! agents author it, and JSON is what a tool-calling agent actually
//! produces). Everything downstream of the AST -- `validate`, `lower`,
//! `interpret`, `resolver`, the `datalog_*` modules -- operates on
//! `ast::Document`'s types and never cared how they were built, so none
//! of it changed when the authoring surface did.

pub mod ast;
pub mod datalog_guard;
pub mod datalog_reachability;
pub mod datalog_validate;
pub mod datalog_worldstate;
pub mod from_json;
pub mod genesis;
pub mod graphview;
pub mod identity;
pub mod interpret;
pub mod lower;
pub mod machine;
pub mod resolver;
pub mod validate;
pub mod view;

pub use ast::Document;
