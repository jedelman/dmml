//! AST for DMML (Desiring-Machine Markup Language), matching the EBNF
//! grammar in `SPEC.md` section 10 ("Formal grammar (EBNF)") exactly.
//! `machine`'s body is kept as an opaque, balanced-brace token span at
//! THIS layer (see `MachineStmt`) -- not because #50 Tier 2 is still
//! unsettled (it isn't; see `MACHINE_SPEC.md`), but because its own
//! grammar is parsed as a deliberate second pass
//! (`crate::machine::parse_machine_body`) over that raw string, rather
//! than folded into this top-level recursive-descent parser.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
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

/// Grammar-reserved, not specified. A parser implementing this grammar
/// accepts the production, consumes a balanced-brace block, and does not
/// attempt to give the interior any structure -- see the grammar's own
/// note. `body` is the raw source text between (not including) the
/// braces, exactly as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineStmt {
    pub node: NodeRef,
    pub body: String,
    pub span: Span,
}
