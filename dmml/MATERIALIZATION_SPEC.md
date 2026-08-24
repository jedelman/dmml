# Retraction-aware materialization (issue #70)

What replaces `crate::interpret::Materialized`'s produces-only fold: a
real fold over commits that carry their own stable identity, where
`consumes` actually retracts, reusing the already-proven
`resolver::factref_matches`/`WorldState` semantics rather than
reinventing them.

## Why commits need identity now

`LoweredCommit` has none of its own — it's the *input* to
`identity::compute_cid`, not a self-addressing value. A `ConsumeRef`
(`Strong` or `Fact`) references a prior commit by `{uri, cid}`
(`StrongRef`), so folding retraction needs a way to look a referenced
`{uri, cid}` up against the commits actually being folded. `uri` isn't
derivable from a commit's own content (it's chosen by whoever publishes
it — an `at://did/collection/rkey`), so it has to be supplied
externally, paired with the commit, the same way `identity::TripleRef`
pairs a triple's content hash with an externally-supplied `owner_did`
rather than baking it into the hash.

```rust
pub struct IdentifiedCommit {
    pub uri: String,
    pub cid: String,
    pub commit: LoweredCommit,
}
```

## The fold

`Materialized::from_identified_commits(commits: &[IdentifiedCommit]) ->
Materialized`, walking `commits` in order. For each commit, in this
order (retraction before assertion — matches `WorldState::
apply_combined_commit`'s own retract-then-assert convention, so a
commit that both consumes an old value and produces its replacement in
one step behaves as a single atomic update, not two separately-ordered
effects):

1. **Apply `consumes`.** For each `ConsumeRef` in `commit.consumes`, in
   order:
   - Find the **target commit**: the entry in `commits` (the *whole*
     slice, not just commits processed so far — a `consumes` always
     references something earlier in real use, but this fold doesn't
     itself enforce commit ordering; that's `resolver`'s job, already
     proven separately) whose `uri` and `cid` both equal the
     `ConsumeRef`'s own `StrongRef` (`ConsumeRef::Strong`'s own field,
     or `ConsumeRef::Fact`'s `.commit` field).
   - **No target found** (dangling `uri`/`cid`) → no-op for this
     `ConsumeRef`. Matches `commit_valid_despite_dangling_factref`'s
     framing: a dangling reference is an ordinary condition, not a
     structural violation — it just retracts nothing.
   - **`ConsumeRef::Strong`**: retract every `(subject, predicate)` the
     target commit's own `produces` ever asserted — i.e. treat a
     whole-commit consume as retracting everything that commit
     produced. (Not from `MATERIALIZATION_SPEC.md`'s own worked
     examples alone — this is this spec's own resolved interpretation
     of `SPEC.md`'s "a commit with no assertions is a pure retraction,"
     generalized to "consuming a whole commit retracts everything it
     asserted"; no other consequence would leave a whole-commit
     `consumes` meaning anything at the triple level.)
   - **`ConsumeRef::Fact`**: look up `(fr.subject, fr.predicate)`
     against the **target commit's own `produces`** (not the running
     fold's current state — a `FactRef` pins to what that specific
     prior commit itself asserted, fixed at authoring time). If the
     target commit's `produces` has no triple for that
     `(subject, predicate)` → dangling, no-op (same fails-open
     posture). If it does, with object `actual`: compute `has_object =
     fr.object.is_some()`, `object_equal = fr.object.as_ref() ==
     Some(actual)`, and call the already-proven
     `resolver::factref_matches(has_object, object_equal)` — call it,
     don't reimplement its logic, same convention `validate::
     commit_is_valid` already established for
     `resolver::cross_repo_commit_valid`. If it returns `true`, retract
     `(fr.subject, fr.predicate)` in the running fold: set its current
     value to absent, unconditionally (per `WorldState`'s own
     "retraction is bookkeeping, a second additive record" framing —
     retracting doesn't care what the running fold's current value
     happens to be right now, only whether this `ConsumeRef` itself
     validly matches its target).
2. **Apply `produces`.** Same as today's fold: for each triple, set
   `(subject, predicate)`'s current value to it — last-write-wins,
   unconditionally, including re-asserting a `(subject, predicate)`
   this same commit (or an earlier one) just retracted in step 1. This
   is exactly `apply_combined_commit`'s atomic pair, generalized from
   one `(retract_key, assert_key)` to a whole commit's worth of
   consumes+produces.

`current_value(subject, predicate)` returns `None` for a retracted (and
not since re-asserted) pair, same as it already returns `None` for a
pair nothing ever asserted — a caller can't distinguish "never
asserted" from "asserted then retracted" through this method, which is
correct: neither is currently true, and nothing about *why* a
`(subject, predicate)` isn't currently true is `Materialized`'s job to
carry.

## Target shape

```rust
pub struct IdentifiedCommit {
    pub uri: String,
    pub cid: String,
    pub commit: LoweredCommit,
}

impl Materialized {
    pub fn from_identified_commits(commits: &[IdentifiedCommit]) -> Self;
    // from_commits (today's produces-only fold) stays as-is, unchanged
    // -- a caller with no consumes to worry about (most of this
    // crate's own existing tests) has no reason to start supplying
    // fabricated identities.
}
```

## Worked examples

Let `mint` be `IdentifiedCommit { uri: "at://did:plc:aaaa/…/mint", cid:
"bafymint", commit: LoweredCommit { produces: [room/42 locked true],
consumes: [], … } }`.

### 1. Strong consume retracts everything the target produced

`update = IdentifiedCommit { uri: "…/update", cid: "bafyupdate", commit:
LoweredCommit { consumes: [ConsumeRef::Strong(StrongRef { uri:
"at://did:plc:aaaa/…/mint", cid: "bafymint" })], produces: [], … } }`.
`from_identified_commits(&[mint, update])`
`.current_value("room/42", "locked")` → `None` — `update` consumed the
whole `mint` commit, which is exactly (and only) the one triple
`room/42 locked true`.

### 2. Fact consume with a matching object retracts just that pair

`update = … consumes: [ConsumeRef::Fact(FactRef { commit: StrongRef {
uri: "…/mint", cid: "bafymint" }, subject: "room/42", predicate:
"locked", object: Some(TripleValue::Boolean(true)) })], produces: [] …`.
Same result as example 1 for `("room/42", "locked")` → `None` — the
object `Some(Boolean(true))` matches what `mint` actually asserted, so
`factref_matches(true, true)` is `true`.

### 3. Fact consume with a non-matching object retracts nothing

Same as example 2, but `object: Some(TripleValue::Boolean(false))`
(`mint` actually asserted `true`, not `false`). `factref_matches(true,
false)` is `false` → no-op.
`.current_value("room/42", "locked")` → still `Some(Boolean(true))`,
untouched.

### 4. Dangling FactRef fails open, doesn't error, doesn't retract

`update` consumes a `FactRef` whose `commit` is `{ uri: "…/nonexistent",
cid: "bafynope" }` — no such commit in the slice.
`from_identified_commits` still returns a `Materialized` (never panics,
never returns `Result`); `.current_value("room/42", "locked")` is
unaffected by this dangling consume, exactly as if it had been omitted.

### 5. Retract-then-reassert in the SAME commit is a net update, not a net retraction

`combined = IdentifiedCommit { uri: "…/combined", cid: "bafycombined",
commit: LoweredCommit { consumes: [ConsumeRef::Fact(FactRef { commit:
StrongRef { uri: "…/mint", cid: "bafymint" }, subject: "room/42",
predicate: "locked", object: None })], produces: [room/42 locked
false], … } }`. `from_identified_commits(&[mint, combined])`
`.current_value("room/42", "locked")` → `Some(Boolean(false))` — step 1
retracts it (object `None` wildcard-matches per `factref_matches(false,
_) == true`), step 2 immediately re-asserts it to `false` — net effect
is the update, matching `apply_combined_commit`'s atomic
retract-then-assert pair, not a moment where the fact is absent.

## Not fully worked, stated only in prose above — testing generalization, not example-matching

- A `ConsumeRef::Strong` referencing a target commit whose `produces` is
  empty (a commit that only ever consumed, never produced — "a pure
  retraction," per `SPEC.md`) retracts nothing (there's nothing in its
  `produces` to iterate) — not an error, not a special case, just zero
  iterations of an already-general rule.
- `commits` containing two `IdentifiedCommit`s with the same `(uri,
  cid)` pair (a malformed input a real repo could never produce, but
  this fold doesn't validate uniqueness) — target lookup should use the
  *first* match found in the slice, same as every other "first match in
  document/slice order" convention already established elsewhere in
  this crate (`validate_same_repo_consumes`'s index ordering,
  `may_fire`'s ident lookup).
