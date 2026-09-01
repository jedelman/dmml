# DMML Surface — a new text authoring syntax (spike, not shipped)

**Status: spike, 2026-08-31. Commits only — no `machine`, `reference`, or
batching yet.** Not a revival of the retired text grammar (`SPEC.md` §10's
old EBNF) — that one is gone and stays gone; this is a new design,
parsed by a new parser (`DMML.Surface`, `dmml-hs`), targeting the same
validated `DMML.Ast.CommitStmt` the JSON front-end (`DMML.FromJson`)
already builds. **One AST, two front-ends** — `DMML.Surface`'s whole job
is text → the exact same `CommitStmt`/`FactStmt`/`Value`/etc. that
`commitStmtFromInput` builds from JSON, so every existing validator
(`isValidIdent`, `isValidNodeRef`, the duplicate-fact check) is reused
verbatim, not reimplemented.

**Why try text again after JSON tested well**: not settled by the
2026-08-31 JSON compliance checkpoint (15/15) — that checkpoint says
JSON-via-`GRAMMAR.md` works, it doesn't say a *better-designed* text
surface wouldn't also work, or work better. The original retired text
grammar was an ad hoc block syntax nobody deliberately optimized for
what a tool-calling model already writes fluently; real, heavily-trained
Haskell syntax is a different bet — worth its own checkpoint, not
assumed either way. That checkpoint hasn't been run yet; see "Not done
yet" below.

## Design principles

- **Lean on real Haskell conventions a model has actually seen a lot
  of**, not an invented-from-scratch syntax: infix backtick application,
  `::` for type-of (a direct, honest parallel to DMML's own `a`/`rdf:type`
  sugar — "`room/1 :: a Room`" reads exactly as it means), record-dot
  field access for plain facts.
- **Let lexical shape carry the tag** that JSON's `{"kind": ...}` had to
  spell out explicitly: a bare, unquoted, slash-shaped token is a node
  reference; a quoted string is a string literal; a bare numeral is a
  number literal; `true`/`false` are booleans. No explicit discriminant
  needed in the surface syntax — the grammar itself disambiguates,
  same information, less to write.
- **No general offside-rule layout algorithm (yet).** Real Haskell's
  layout rule is a real parser project on its own; this spike uses a
  single fixed indentation level per block instead of full recursive
  layout, to stay a spike. If nested constructs need real layout later
  (a machine's guards/effects, most likely), that's separate follow-up,
  not assumed solved here.

## Grammar (informal, commits only)

```
commit <verb>
  declare relation <ident>
  declare attribute <ident>

  <node_ref> :: a <node_ref>
  <node_ref> `<ident>` <value>
  <node_ref> . <ident> = <value>

  consumes
    strong <uri> # <cid>
    fact <uri> # <cid>
      <node_ref> . <ident>
      <node_ref> . <ident> = <value>

  refs
    <role_ident> <uri> # <cid>
```

- `<verb>`, `<ident>`, `<role_ident>` — same lexical class as JSON's
  bare identifiers (`isValidIdent`, reused directly).
- `<node_ref>` — same `segment(/segment)*` shape as JSON's node
  references (`isValidNodeRef`, reused directly).
- `<value>` — a `<node_ref>` (bareword), a quoted string (`"..."`), a
  bare numeral (`0.6`, `3`), or `true`/`false`.
- `<node_ref> :: a <node_ref>` — sugar for the `"a"`/`rdf:type` fact,
  matching JSON's own `predicate_ref = "a" | ident` distinction.
- `` <node_ref> `<ident>` <value> `` — infix backtick application: a
  plain relation/attribute fact, subject-predicate-object in reading
  order (`room/1 \`opensTo\` room/2`).
- `<node_ref> . <ident> = <value>` — dot-field-assignment form of the
  same fact shape; both forms lower to the identical `FactStmt`, purely
  a style choice for whichever reads better for a given predicate (infix
  verbs like `opensTo`; dot-assignment for attribute-like predicates
  like `dampness`).
- `consumes`/`fact ... # ...` sub-block's dot line with no `=` is the
  wildcard-object omission (same semantics as JSON's omitted `object`
  field); with `=` it's an explicit object, same as JSON's present
  `object` field.
- `#` separates a `<uri>` from its `<cid>` in both `consumes` and
  `refs` — evokes a fragment/anchor, not reused from anywhere else in
  this grammar.

## Example

```
commit mints
  declare relation opensTo
  declare attribute dampness

  room/1 :: a Room
  room/2 :: a Room
  room/1 `opensTo` room/2
  room/1 . dampness = 0.6
```

Parses to the identical `CommitStmt` the JSON authoring example in
`GRAMMAR.md` produces for the same content.

## What's implemented

`src/DMML/Surface.hs` — a `megaparsec` parser, `parseCommitSurface ::
Text -> Either (ParseErrorBundle Text Void) CommitStmt`. Verified by
parsing the example above and diffing the resulting `CommitStmt`
against the one `DMML.FromJson.commitFromJson` builds from the
equivalent JSON — same `Ast.CommitStmt` value, two different front-ends,
checked to actually agree rather than assumed to.

## What's NOT done yet — real gaps, not implementation debt to ignore

- ~~No compliance checkpoint against light models yet.~~ **Done,
  2026-09-01: 15/15 accepted** (`dmml/compliance-surface/`, same three
  models and the same discipline as the JSON checkpoint — a real oracle,
  `ComplianceCheckSurface.hs`, calling `parseCommitSurface` directly).
  Matches the JSON checkpoint's own 15/15 exactly. Doesn't settle
  "Haskell-styled text is *better* than JSON for agents" (nothing here
  compares the two head-to-head on the same failure modes) — only that
  it's not *worse*, on this scenario set, against these three models.
  One real, checkpoint-scoped-only finding worth carrying forward: since
  this parser checks shape only (see below), a reply that uses a
  predicate without declaring it (observed once, Kimi) still scores
  "accepted" here — that gap won't be visible until self-declaration
  semantics exist to check against.
- **No `machine`, `reference`, or batching (`update`) support.** JSON's
  `MachineInput`/`ReferenceInput`/`UpdateInput` have no surface-syntax
  counterpart here at all.
- **No error-recovery/reporting design.** `megaparsec`'s default error
  messages are used as-is — JSON Pointer-style localized errors
  (`from_json.rs`'s whole design point) don't have an equivalent here.
- **No layout algorithm** — see "Design principles" above.
