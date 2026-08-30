# DMML lowering spec — `commit` blocks and `reference` statements

What `dmml::lower::lower_commit`/`lower_reference` implement.
`machine_stmt` stays out of scope (grammar-reserved, not specified —
`SPEC.md` itself hasn't settled it, see `dmml/src/lower.rs`'s module doc
comment).

## Target shape

`LoweredCommit { predicate_verb, consumes: Vec<ConsumeRef>, produces:
Vec<Triple>, refs: HashMap<String, Vec<StrongRef>> }` — see
`dmml/src/lower.rs` for the exact types. Self-contained: `produces` is a
flat `Vec<Triple>`, not serialized N-Quads text, since dmml has no
N-Quads writer of its own.

`refs` replaced two dedicated `via: Option<StrongRef>`/`responds_to:
Option<StrongRef>` fields (2026-08-29): every commit-level reference is
now a role-tagged entry in one open map (`"via"`, `"respondsTo"`,
`"requires"`, ... — see `ast::CommitStmt.refs`'s own doc comment for why
every role is a real `Vec`, never a single last-wins value). `requires`
is the newest role: a list of commits this one depends on, checked by
`interpret::requires_are_valid` against a real history (not by
`lower_commit` itself, which has no history to check against) and folded
into `resolver::commit_is_valid`'s validity result — see that function's
own doc comment for the deliberate, on-the-record break of its prior
formal-verification status this required.

## Rules

1. `predicate_verb` is copied verbatim from the parsed `commit`'s own
   `predicate_verb`.
2. A `NodeRef` lowers to its segments joined with `/` (`["room","42"]` →
   `"room/42"`).
3. A `PredicateRef` lowers to `"rdf:type"` for `PredicateRef::RdfType`
   (the `a` shorthand), or the ident text for `PredicateRef::Ident(s)`.
4. A `Value` lowers to a `TripleValue`: `Value::Node(n)` →
   `TripleValue::Node(<rule 2>)`; `Literal::Number(s)` →
   `TripleValue::Number(s)`; `Literal::Boolean(b)` →
   `TripleValue::Boolean(b)`; `Literal::String(s)` → `TripleValue::Str(s)`.
5. An `ast::StrongRef` lowers to this module's `StrongRef { uri:
   ast_sr.uri.raw, cid: ast_sr.cid }`.
6. A `DeclareStmt { kind, ident }` lowers to exactly one `Triple`:
   `subject: ident`, `predicate: "rdf:type"`, `object:
   TripleValue::Node("Relation")` or `TripleValue::Node("Attribute")`
   depending on `kind` — mirrors `SPEC.md`'s `declare relation X` → `<X>
   rdf:type ww:Relation .`, simplified: no IRI namespace machinery
   (`vocab::dynamic_predicate`/`ww:` prefix), plain idents/literal
   strings stand in for it. A real production lowering would need that
   machinery; this reference lowering deliberately doesn't reimplement
   it.
7. A `FactStmt { subject, predicate, value }` lowers to exactly one
   `Triple`: `subject: <rule 2>`, `predicate: <rule 3>`, `object: <rule
   4>`.
8. Walk `commit.items` in document order:
   - `Declare(d)` / `Fact(f)` → lower per rule 6/7, push onto `produces`.
   - `Produces(block)` → lower every fact in `block.facts` per rule 7,
     push each onto `produces`, in order.
   - `Consumes(block)` → for each entry: `Strong(sr)` → push
     `ConsumeRef::Strong(<rule 5>)`; `Fact(fc)` → push
     `ConsumeRef::Fact(FactRef { commit: <rule 5 on fc.commit>, subject:
     <rule 2 on fc.subject>, predicate: fc.predicate, object:
     fc.object.map(<rule 4>) })`.
9. `commit.refs` (a `HashMap<String, Vec<ast::StrongRef>>`, populated
   directly by `from_json` rather than walked out of `commit.items` --
   see `ast::CommitStmt.refs`'s own doc comment) lowers role-by-role:
   each `(role, targets)` entry becomes `(role.clone(), targets.iter().
   map(<rule 5>).collect())` in the output `refs` map. Every entry under
   a role is kept, in order -- there is no last-wins collapsing anymore,
   since a role is a real list now, not a single value a repeated item
   could overwrite.
10. A bare `Declare`/`Fact` item and a `Produces` block's facts are NOT
   distinguished in the output — both contribute to the SAME flat
   `produces`, in the exact order encountered walking `commit.items`.
   This is `SPEC.md`'s "sugar for implicit produces block" rule, made
   concrete: the surface form the author chose doesn't survive lowering.

## Worked examples (exact expected output, verified against the real parser)

### 1. Declare-then-assert (mint)

```dmml
commit mints {
  declare relation opensTo
  declare attribute dampness

  room/42 a Room
  room/42 opensTo room/43
  room/42 dampness 0.4
}
```

```rust
LoweredCommit {
    predicate_verb: "mints".to_string(),
    consumes: vec![],
    produces: vec![
        Triple { subject: "opensTo".to_string(), predicate: "rdf:type".to_string(), object: TripleValue::Node("Relation".to_string()) },
        Triple { subject: "dampness".to_string(), predicate: "rdf:type".to_string(), object: TripleValue::Node("Attribute".to_string()) },
        Triple { subject: "room/42".to_string(), predicate: "rdf:type".to_string(), object: TripleValue::Node("Room".to_string()) },
        Triple { subject: "room/42".to_string(), predicate: "opensTo".to_string(), object: TripleValue::Node("room/43".to_string()) },
        Triple { subject: "room/42".to_string(), predicate: "dampness".to_string(), object: TripleValue::Number("0.4".to_string()) },
    ],
    refs: HashMap::new(),
}
```

### 2. Consumes + produces (becomes)

```dmml
commit becomes {
  consumes {
    fact at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789
      (cid: bafyabcxyz) {
      subject: room/42
      predicate: locked
    }
  }
  produces {
    room/42 locked false
  }
}
```

```rust
LoweredCommit {
    predicate_verb: "becomes".to_string(),
    consumes: vec![
        ConsumeRef::Fact(FactRef {
            commit: StrongRef { uri: "at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789".to_string(), cid: "bafyabcxyz".to_string() },
            subject: "room/42".to_string(),
            predicate: "locked".to_string(),
            object: None,
        }),
    ],
    produces: vec![
        Triple { subject: "room/42".to_string(), predicate: "locked".to_string(), object: TripleValue::Boolean(false) },
    ],
    refs: HashMap::new(),
}
```

### 3. via / respondsTo / requires (grants)

JSON authoring shape (`CommitInput.refs`, not `commit_item`s -- see rule 9):

```json
{
  "verb": "grants",
  "refs": {
    "via": [{"uri": "at://did:plc:abc/org.foo.bar/rkey1", "cid": "bafyxyz1"}],
    "respondsTo": [{"uri": "at://did:plc:def/org.foo.bar/rkey2", "cid": "bafyxyz2"}],
    "requires": [{"uri": "at://did:plc:ghi/org.foo.bar/rkey3", "cid": "bafyxyz3"}]
  }
}
```

```rust
LoweredCommit {
    predicate_verb: "grants".to_string(),
    consumes: vec![],
    produces: vec![],
    refs: HashMap::from([
        ("via".to_string(), vec![StrongRef { uri: "at://did:plc:abc/org.foo.bar/rkey1".to_string(), cid: "bafyxyz1".to_string() }]),
        ("respondsTo".to_string(), vec![StrongRef { uri: "at://did:plc:def/org.foo.bar/rkey2".to_string(), cid: "bafyxyz2".to_string() }]),
        ("requires".to_string(), vec![StrongRef { uri: "at://did:plc:ghi/org.foo.bar/rkey3".to_string(), cid: "bafyxyz3".to_string() }]),
    ]),
}
```

## Scope experiment note

This spec was written in full *before* dispatching the implementation to
Kimi (moonshotai/kimi-k2.5) in a single shot — the whole `lower_commit`
function plus helpers, no back-and-forth. See dev-journal for the result
and how much survived unmodified.

---

# Reference statement lowering

`reference <at-uri> (cid: ...) [as <local-name>]` — `SPEC.md` SS6/SS10:
"lowers to an ordinary `produces` triple (`foreignUri`/`foreignCid` — the
`reach`-style pattern already in production, formalized as grammar) — no
repo check, ever, on a separate code path from `consumes`." Corroborated
directly against `engine/src/vocab.rs`'s real `foreign_uri()`/
`foreign_cid()` predicates (`"foreignUri"`/`"foreignCid"`) and
`README.md`'s "Corruption as content" section: `reach` links a room to a
foreign atproto record via ordinary `foreignUri`/`foreignCid` predicates
**on the room's own node** — never through `consumes`, never claiming
authority over the foreign content, only watching it.

## Target shape

```rust
pub fn lower_reference(reference: &ast::ReferenceStmt) -> Vec<Triple>
```

Not folded into `LoweredCommit` — a top-level `reference` isn't inside a
`commit` block at all (see the grammar: `top_level_item = commit_stmt |
reference_stmt | machine_stmt`, siblings, not nested). Returns the
triples a caller can fold into a materialized view alongside a commit
log's own triples — wiring that fold together is explicitly a follow-up,
not attempted here.

## Rules

1. **The `as <local-name>` clause is the subject the triples attach to**
   — confirmed against `README.md`'s "Corruption as content" section
   (`reach` attaches to "the room's own" node, never an anonymous one)
   and `SPEC.md`'s own worked example (`... as room/42.reach`).
2. **When `as_name` is `Some(node_ref)`**, produces exactly two triples,
   in this order:
   - `Triple { subject: <rule 2 from the commit-lowering spec, node_ref's
     segments joined with "/">, predicate: "foreignUri", object:
     TripleValue::Str(target.uri.raw) }`
   - `Triple { subject: <same subject>, predicate: "foreignCid", object:
     TripleValue::Str(target.cid) }`

   Both values are `TripleValue::Str`, not `TripleValue::Node` — an
   `at://` URI and a CID are opaque foreign-system strings, not shaped
   like this grammar's own `node_ref` segments, so treating them as
   plain string literals is more honest than forcing them into the
   `Node` variant DMML-authored node references use.
3. **When `as_name` is `None` (the clause is grammatically optional —
   `SPEC.md`'s own EBNF: `[ "as" , node_ref ]`), `SPEC.md` does not say
   what happens.** Every real, worked example this session has seen
   attaches `foreignUri`/`foreignCid` to a specific node — there is no
   established convention for an anonymous/subject-less reference. This
   spec resolves the gap explicitly, not silently: **`lower_reference`
   returns an empty `Vec` when `as_name` is `None`** — a reference with
   no subject to attach to produces no triples, rather than guessing at
   an anonymous or synthetic subject. Flagged here as an interpretation
   of an underspecified case, same as `LOWERING_SPEC.md` rule 6's own
   IRI-namespace gap above — not asserted as the only correct reading.

## Worked examples (exact expected output)

### 1. `as` given — two triples, in order

```dmml
reference at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456
  (cid: bafyqrs456) as room/42.reach
```

```rust
vec![
    Triple {
        subject: "room/42.reach".to_string(),
        predicate: "foreignUri".to_string(),
        object: TripleValue::Str("at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456".to_string()),
    },
    Triple {
        subject: "room/42.reach".to_string(),
        predicate: "foreignCid".to_string(),
        object: TripleValue::Str("bafyqrs456".to_string()),
    },
]
```

### 2. `as` omitted — empty result (rule 3)

```dmml
reference at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456
  (cid: bafyqrs456)
```

```rust
vec![]
```

Not fully worked, stated only as rule 2's ordering language ("in this
order") — testing generalization, not example-matching: a reference
whose `as`-name has multiple segments (e.g. `as key/7.reach`) must still
produce exactly two triples, both sharing that same joined subject
(`"key/7.reach"`), `foreignUri` strictly before `foreignCid` in the
returned `Vec`'s order.

