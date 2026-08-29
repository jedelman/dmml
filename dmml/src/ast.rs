//! AST for DMML (Desiring-Machine Markup Language). Built directly from
//! JSON authoring input (`crate::from_json`) -- there is no text grammar
//! and no text parser; a hand-written DMML source language was retired
//! (see `from_json`'s own doc comment) once JSON became the only real
//! authoring surface, since nothing hand-writes DMML source text anymore
//! and JSON is what a tool-calling agent actually produces. `MachineStmt`
//! carries its states/transitions as real structured data for the same
//! reason -- there is no longer a raw balanced-brace body to keep opaque
//! or a second parsing pass to defer to; `MACHINE_SPEC.md` still governs
//! the *semantics* of states/transitions/guards/effects, just not their
//! surface syntax.

/// A JSON Pointer (RFC 6901) into the authoring request payload this AST
/// node was built from -- e.g. `/facts/2/predicate`. Stands in for the
/// byte-range-into-source-text `Span` this crate used before the DSL's
/// text parser was retired: same job (let an error point at where in the
/// original request a problem came from), different coordinate space,
/// since there is no source text anymore to take a byte range into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub pointer: String,
}

impl Span {
    pub fn new(pointer: impl Into<String>) -> Self {
        Span { pointer: pointer.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub items: Vec<TopLevelItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopLevelItem {
    Commit(CommitStmt),
    Reference(ReferenceStmt),
    Machine(MachineStmt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitStmt {
    /// Open vocabulary: `mints`, `becomes`, `divides`, `grants`, ... --
    /// never validated against a closed enum, per the grammar's own note.
    pub predicate_verb: String,
    pub items: Vec<CommitItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitItem {
    /// A bare `declare_stmt` or `fact_stmt` appearing directly in a
    /// `commit` body (outside any `produces { }` block) is sugar for
    /// "implicit produces block" -- lowered identically to one inside an
    /// explicit block. Kept as a distinct variant here (not pre-merged
    /// into `Produces`) so a consumer can tell which surface form the
    /// author actually used, per SPEC.md's note that both forms parse to
    /// the same production.
    Declare(DeclareStmt),
    Fact(FactStmt),
    Produces(ProducesBlock),
    Consumes(ConsumesBlock),
    Via(StrongRef),
    RespondsTo(StrongRef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclareStmt {
    pub kind: DeclKind,
    pub ident: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    Relation,
    Attribute,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducesBlock {
    pub facts: Vec<FactStmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactStmt {
    pub subject: NodeRef,
    /// `"a"` is Turtle-style sugar for `rdf:type`, same convention as the
    /// rest of this design's N-Quads lowering.
    pub predicate: PredicateRef,
    pub value: Value,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicateRef {
    RdfType,
    Ident(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Node(NodeRef),
    Literal(Literal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Number(String),
    Boolean(bool),
    String(String),
}

/// `ident , { "/" , ident }` -- e.g. `room/42`, `key/7`. Stored as its
/// literal segments, not pre-joined, so a consumer can inspect the shape
/// without re-splitting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRef {
    pub segments: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumesBlock {
    pub entries: Vec<ConsumeEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumeEntry {
    Strong(StrongRef),
    Fact(FactConsume),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactConsume {
    pub commit: StrongRef,
    pub subject: NodeRef,
    pub predicate: String,
    /// Omitted `object` preserves `FactRef`'s existing wildcard semantics
    /// -- every triple asserted for `(subject, predicate)`.
    pub object: Option<Value>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrongRef {
    pub uri: AtUri,
    pub cid: String,
    pub span: Span,
}

/// `"at://" , did , "/" , nsid , "/" , rkey` -- atproto's own AT-URI
/// syntax, reused verbatim, not reinvented here. Stored as the raw text
/// plus its three parsed segments; the grammar does not validate DID/NSID/
/// rkey shape beyond "non-empty, slash-delimited" -- that's atproto's own
/// concern, not DMML's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtUri {
    pub raw: String,
    pub did: String,
    pub nsid: String,
    pub rkey: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceStmt {
    pub target: StrongRef,
    pub as_name: Option<NodeRef>,
    pub span: Span,
}

/// A desiring-machine declaration: the node it's attached to, plus its
/// states and transitions per `MACHINE_SPEC.md`. Structured data directly
/// from JSON authoring input -- no opaque body, no second parsing pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineStmt {
    pub node: NodeRef,
    pub states: Vec<crate::machine::StateDecl>,
    pub transitions: Vec<crate::machine::TransitionDecl>,
    pub span: Span,
}
