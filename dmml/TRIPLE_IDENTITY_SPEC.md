# Triple identity spec — content-addressed triples, DID-paired

What `dmml::identity`'s triple-identity functions implement, per this
session's design conversation: each `Triple` gets its own content CID
(not just each commit); a commit that touches multiple triples is a
*batch*, not the unit of identity. Sovereignty (a repo's own unconditional
write authority) is what makes this safe without extra machinery — see
"Why this is safe" below.

## The core design decision

**A triple's CID is a pure content hash of `(subject, predicate, object)`
alone.** No DID, no commit context, no timestamp goes into the hash. This
is deliberate, not an oversight — it directly mirrors how atproto itself
works: a record's CID is *always* a pure content hash of the record's own
bytes; ownership/location is *never* baked into the hash, it's carried
*alongside* it (a `com.atproto.repo.strongRef` is `{ uri, cid }` — the
`uri` carries the DID+location, `cid` is pure content addressing, kept
deliberately separate).

**Ownership is expressed by pairing the triple CID with an explicit
`owner_did`, in a new type, `TripleRef`** — mirroring `StrongRef`'s own
shape rather than inventing a new pattern:

```rust
pub struct TripleRef {
    pub owner_did: String,
    pub triple: Cid,
}
```

## Consequences, worked through

1. **Same repo, same content, any time → same triple CID.** Because the
   hash is pure content, a repo re-asserting `(subject, predicate,
   object)` it already asserted before gets the *identical* CID — not a
   new, distinct triple. This is intentional: it's the same fact, so it's
   the same triple. (This also matches `dmml::resolver`'s existing
   `assert_fact` being idempotent — same conclusion, arrived at
   independently earlier this session, now explained by the identity
   scheme rather than just assumed.)
2. **Different repos, same content → same triple CID, different
   `TripleRef`.** Two repos independently asserting `room/42 a Room`
   produce the *same* `triple_cid` (pure content, no DID) but *different*
   `TripleRef`s (different `owner_did`), because they're paired with
   different owners. The CID alone can't tell you who asserted something;
   the `TripleRef` pair can.
3. **Different content, same repo → different triple CID.** Ordinary
   content-addressing: any difference in `subject`, `predicate`, or
   `object` changes the hash.

## Why this is safe: sovereignty carries the load

Same-repo-only retraction (`SPEC.md`, already-proven
`fact_retraction_fails_open.th`/`cross_repo_consume_fails_closed.th`)
needs a check: "is this triple actually mine to retract?" Because a
triple CID alone is ambiguous across repos (consequence 2 above), that
check can't be "does this CID exist" — it has to be "does *my own* log
contain a commit that asserted a triple with this CID." This is exactly
**repo-local determinism** (already proven this session,
`repo_local_determinism.th`): a repo only ever needs its *own* commit
log to answer this, no cross-repo lookup, ever. A repo naturally
maintains its own local index of "triple CIDs I've minted" (it's the one
that asserted them), so the check is a local lookup, not a network
round-trip or a trust decision about a foreign repo's claims.

## Functions to implement

All in `dmml::identity`, alongside the existing `compute_cid` (reuse its
codec/hash constants — `DAG_CBOR_CODEC = 0x71`, `SHA2_256_CODE = 0x12`,
the same `Multihash<64>`/`Cid::new_v1` pipeline — for consistency, not a
second encoding scheme).

1. `pub fn triple_cid(triple: &crate::lower::Triple) -> cid::Cid` —
   `CIDv1(dag-cbor, sha2-256)` over `triple` directly (it now derives
   `Serialize` — see `lower.rs`), no DID, no wrapping struct needed. Same
   `serde_ipld_dagcbor::to_vec` → `Sha256::digest` → `Multihash::<64>::
   wrap` → `Cid::new_v1` pipeline as `compute_cid`, just over a much
   smaller value.
2. `pub struct TripleRef { pub owner_did: String, pub triple: cid::Cid }`
   — plain data, `Debug + Clone + PartialEq + Eq`.
3. `pub fn make_triple_ref(owner_did: &str, triple: &crate::lower::
   Triple) -> TripleRef` — trivial constructor: `TripleRef { owner_did:
   owner_did.to_string(), triple: triple_cid(triple) }`.
4. `pub fn triple_ref_matches(reference: &TripleRef, owner_did: &str,
   triple: &crate::lower::Triple) -> bool` — the actual verification a
   resolver runs before honoring a retraction: recompute `triple_cid
   (triple)` and check it equals `reference.triple`, **AND** check
   `reference.owner_did == owner_did`. **Both must hold** — a triple CID
   match alone is NOT sufficient (consequence 2: two different owners can
   share a triple CID), and a DID match alone is meaningless without the
   content check. Neither check is optional or "close enough."

## Worked examples (exact expected values)

Let `t1 = Triple { subject: "room/42".into(), predicate: "locked".into(),
object: TripleValue::Boolean(true) }`, `t2 = Triple { subject:
"room/42".into(), predicate: "locked".into(), object:
TripleValue::Boolean(false) }` (same subject/predicate, different
object), `did_a = "did:plc:aaaa1111"`, `did_b = "did:plc:bbbb2222"`.

1. `triple_cid(&t1) == triple_cid(&t1)` — calling it twice on equal
   content gives the identical CID (determinism).
2. `triple_cid(&t1) != triple_cid(&t2)` — different `object` values give
   different CIDs.
3. `make_triple_ref(did_a, &t1).triple == make_triple_ref(did_b,
   &t1).triple` — the triple-CID half is identical regardless of owner
   (consequence 2), but the two `TripleRef`s as a whole are NOT equal
   (`owner_did` differs).
4. `triple_ref_matches(&make_triple_ref(did_a, &t1), did_a, &t1) ==
   true` — the straightforward matching case.
5. `triple_ref_matches(&make_triple_ref(did_a, &t1), did_b, &t1) ==
   false` — same triple content, WRONG owner_did given: must fail. This
   is the actual same-repo-enforcement check, and it's the one that must
   never be gotten wrong.

Not fully worked, stated only as rule 4's own "both must hold" language —
testing generalization, not example-matching, same as this session's
earlier spec-first dispatches:
- `triple_ref_matches(&make_triple_ref(did_a, &t1), did_a, &t2)` must be
  `false` (right owner, WRONG content — content changed, so the
  recomputed CID no longer matches `reference.triple`).
