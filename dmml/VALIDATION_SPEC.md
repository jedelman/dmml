# DMML self-declaration validation spec — single-commit scope

What `dmml::validate::validate_declarations` implements: `SPEC.md` SS3's
two-tier predicate rule (closed/structural vs. open/self-declared),
scoped to checking one parsed `commit { ... }` block's own items.

## The rule

1. There is exactly one **closed** predicate: `rdf:type` (the `a`
   shorthand lowers to this — see `PredicateRef::RdfType` in
   `crate::ast`). A fact using it is always valid; it is never checked
   against declarations.
2. Every OTHER predicate used by a fact (`PredicateRef::Ident(s)`) must
   be **self-declared**: some `DeclareStmt` with `ident == s` must appear
   *somewhere* in the same commit's `items` — a `declare relation <s>` or
   `declare attribute <s>` (the `DeclKind` — `Relation` vs. `Attribute` —
   does not matter for this check; either kind of declaration makes `s`
   declared).
3. **Order-independent within the commit.** A fact using a predicate may
   appear *before* the `declare` that declares it, textually — this
   mirrors the real engine's `validate_self_declared`, which is
   "commit-batch-sensitive, not line-order-sensitive": collect every
   declared ident first (from anywhere in `commit.items`), then check
   every use against the whole collected set, regardless of where in the
   document each item appeared.
4. A bare `CommitItem::Fact` and a fact inside an explicit
   `CommitItem::Produces` block's `ProducesBlock.facts` are checked
   identically — both are "uses of a predicate," full stop, matching
   `crate::lower`'s "sugar for implicit produces block" treatment of the
   same distinction.
5. `FactConsume.predicate` (inside a `CommitItem::Consumes` block) is
   **not** checked by this function at all — a consume references an
   already-established fact from prior history, not a new assertion in
   this commit, so it isn't subject to this commit's own self-declaration
   requirement.
6. On failure, return every undeclared use found — not just the first —
   as a `Vec<UndeclaredPredicate>`, each carrying the offending fact's own
   predicate name and the fact statement's `span` (i.e.
   `FactStmt.span`, not the whole commit's span), **in document order**
   (the order the offending fact statements appear in `commit.items`,
   walking bare facts and `produces` blocks' facts in the same single
   pass, in the order encountered). If a document uses the same
   undeclared predicate more than once, report EACH use separately (do
   not deduplicate) — every offending fact gets its own
   `UndeclaredPredicate` entry.
7. `Ok(())` if every predicate used passes rules 1–2.

## Worked examples (exact expected output)

### 1. Valid: declared before use

```dmml
commit mints {
  declare relation opensTo
  room/42 opensTo room/43
}
```
→ `Ok(())`

### 2. Valid: declared AFTER use (order-independent — rule 3)

```dmml
commit mints {
  room/42 opensTo room/43
  declare relation opensTo
}
```
→ `Ok(())`

### 3. Invalid: never declared

```dmml
commit mints {
  room/42 opensTo room/43
}
```
→ `Err(vec![UndeclaredPredicate { predicate: "opensTo".to_string(), span: <the FactStmt's own span, i.e. the span of the "room/42 opensTo room/43" line> }])`

### 4. `a` / `rdf:type` never needs declaring (rule 1)

```dmml
commit mints {
  room/42 a Room
}
```
→ `Ok(())`

### 5. Multiple undeclared predicates: every use reported, in document order (rule 6)

```dmml
commit mints {
  room/42 opensTo room/43
  room/42 dampness 0.4
}
```
→ `Err(vec![UndeclaredPredicate { predicate: "opensTo".to_string(), span: <span of the first fact> }, UndeclaredPredicate { predicate: "dampness".to_string(), span: <span of the second fact> }])` — two entries, in the order the facts appear, `opensTo` first.

## Not fully worked, stated only in prose (rules 4–5 above) — testing generalization, not example-matching

- A fact inside an explicit `produces { }` block obeys the identical rule
  (rule 4) — no separate worked example for this; it should behave
  exactly like example 3/5 but with the fact nested one level deeper.
- `FactConsume` predicates inside `consumes { }` are exempt entirely
  (rule 5) — a commit with only a `consumes { fact ... { predicate: X } }`
  referencing an undeclared-in-this-commit `X`, and no `produces`, should
  return `Ok(())`.

---

# Same-repo `consumes` structural validation

What `dmml::validate::validate_same_repo_consumes`/`commit_is_valid`
implement: `SPEC.md` SS6's same-repo-only enforcement for `consumes`,
operating on real lowered data and feeding the already-proven
`dmml::resolver::cross_repo_commit_valid` gate
(`cross_repo_consume_fails_closed.th`, L3-certified this session) — this
is the detector that computes the boolean that contract already proved
the *consequence* of.

## The rule

1. Every `ConsumeRef` in a `LoweredCommit.consumes` carries an `at://`
   URI whose authority segment IS the asserting repo's DID — mirrors
   `engine/src/vocab.rs::did_of_at_uri` exactly (same extraction logic,
   duplicated by the same convention `crate::identity::COMMIT_NSID`
   already established: `dmml` doesn't depend on `engine`, so this is a
   plain, independently-implemented helper kept in sync by convention,
   not a shared item). For `ConsumeRef::Strong(sr)`, check `sr.uri`. For
   `ConsumeRef::Fact(fr)`, check `fr.commit.uri` (the commit the
   fact-level retraction targets — the lexicon's own `factRef.commit`
   doc comment: "Must be in the same repo as the commit this factRef is
   carried by").
2. **DID extraction**: given an `at://` URI string, strip the `at://`
   prefix, split on `/`, take the first segment; `None` if the string
   doesn't start with `at://` or that first segment is empty.
   (`LoweredCommit`'s `StrongRef.uri` values come from an already-parsed
   `AtUri`, so this should always succeed in practice — but never
   `unwrap`/panic on it; a `None` extraction is itself treated as a
   violation, fail-closed, not skipped.)
3. **A `ConsumeRef` is a violation** if its extracted DID (per rule 2)
   is `None`, OR is `Some(did)` where `did != authoring_did`.
4. `validate_same_repo_consumes(commit, authoring_did)` returns every
   violation found, **in `commit.consumes`'s own index order** (not
   reordered), each carrying that entry's own index and the foreign DID
   found (or a fixed placeholder string if extraction failed) —
   `Ok(())` if there are none.
5. `commit_is_valid(commit, authoring_did, declarations_ok)` is the tie-
   in to the already-proven contract: computes `is_cross_repo_consume =
   validate_same_repo_consumes(commit, authoring_did).is_err()`, then
   returns `dmml::resolver::cross_repo_commit_valid(is_cross_repo_consume,
   declarations_ok)` — literally calling the proven function, not
   reimplementing its logic.

## Target shape

```rust
pub struct CrossRepoConsume {
    pub index: usize,
    pub foreign_did: String,
}

pub fn validate_same_repo_consumes(
    commit: &crate::lower::LoweredCommit,
    authoring_did: &str,
) -> Result<(), Vec<CrossRepoConsume>>

pub fn commit_is_valid(
    commit: &crate::lower::LoweredCommit,
    authoring_did: &str,
    declarations_ok: bool,
) -> bool
```

## Worked examples (exact expected values)

Let `authoring_did = "did:plc:aaaa1111"`.

### 1. Same-repo `Strong` consume — no violation

`consumes[0] = ConsumeRef::Strong(StrongRef { uri:
"at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789",
cid: "bafyabcxyz" })` (same DID as `authoring_did`) →
`validate_same_repo_consumes` returns `Ok(())`.

### 2. Cross-repo `Strong` consume — one violation

`consumes[0] = ConsumeRef::Strong(StrongRef { uri:
"at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456",
cid: "bafyqrs456" })` (a DIFFERENT DID) →
`Err(vec![CrossRepoConsume { index: 0, foreign_did:
"did:plc:zzzz9999".to_string() }])`.

### 3. Cross-repo `Fact` consume — checks `fr.commit.uri`, not the
   fact's own subject/predicate

`consumes[0] = ConsumeRef::Fact(FactRef { commit: StrongRef { uri:
"at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456",
cid: "bafyqrs456" }, subject: "room/42".to_string(), predicate:
"locked".to_string(), object: None })` → `Err(vec![CrossRepoConsume {
index: 0, foreign_did: "did:plc:zzzz9999".to_string() }])` — the
violation is about `fr.commit.uri`'s DID, `subject`/`predicate` never
enter into this check at all.

### 4. Multiple consumes, mixed — only the violating indices, in order

`consumes = [Strong(same-repo), Strong(cross-repo, did:plc:zzzz9999),
Strong(same-repo), Fact(cross-repo, did:plc:cccc3333)]` →
`Err(vec![CrossRepoConsume { index: 1, foreign_did:
"did:plc:zzzz9999".to_string() }, CrossRepoConsume { index: 3,
foreign_did: "did:plc:cccc3333".to_string() }])` — indices 0 and 2 are
silently fine, not present in the result at all.

### 5. Empty `consumes` — no violations possible

`commit.consumes == vec![]` → `Ok(())`.

## Not fully worked, stated only as rule 5's own language — testing generalization, not example-matching

- `commit_is_valid` given a commit with a cross-repo consume (so
  `validate_same_repo_consumes` returns `Err`) and `declarations_ok:
  true` must still return `false` overall — a cross-repo consume voids
  the whole commit regardless of everything else being fine (this is
  exactly what `cross_repo_consume_fails_closed.th` already proved;
  `commit_is_valid` is only correct if it actually calls that function
  rather than reimplementing similar-looking logic).
