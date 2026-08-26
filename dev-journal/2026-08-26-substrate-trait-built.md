# `Substrate`'s real trait signatures, built (2026-08-26)

Jason: "proceed with implementation per protocol, using zAI/GLM 5.3."
Scoped this honestly first — the full live-deployment build (Android
app, OAuth flows, a real iroh service) is weeks of multi-platform work,
not one session. What's actually implementable now, per the DMML-first/
dispatch protocol, is the one piece everything else in `ARCHITECTURE.md`'s
"Open design work" depends on: the `Substrate` trait, previously a
one-method stub.

## The dispatch

Gave `z-ai/glm-5.3` the full decided architecture (client split,
`consumes` as the only destructive op, `disputes` over arbitration, the
`getResolved` reuse finding) and the actual stub, asked it to design the
real trait shape. Hit the documented failure mode immediately —
reasoning alone burned the entire 4000-token budget, `content: null`.
Not new: written-world's `CLAUDE.md` already names this exact failure
for this model. Fixed by raising `max_tokens` in two steps (4000 → 16000
still truncated mid-file at `finish_reason: length`; 16000 → 32000
finished clean at `finish_reason: stop`). Worth a note for next time:
this model's reasoning volume on genuinely intricate design work is
large enough that 4000 tokens of *total* budget was never going to be
enough regardless of the prompt — start higher next time rather than
stepping up twice.

**The actual design is good and I kept it**, with real adaptation, not
a paste: capability traits, not one enum-shaped write method —
`Substrate` (identity, sovereignty root, reads) plus `CasSubstrate`
(atproto) and `AppendSubstrate` (iroh), each carrying only the write
contract its backend can actually honor. The reasoning holds up: an
enum-shaped alternative would force iroh to carry a `Conflict` variant
it can never produce and atproto to accept an `expected` parameter that
means nothing without a real swap underneath it — "does this backend do
CAS" would become a runtime question instead of a type-level fact.

## Adapting the sketch to the real codebase

GLM's sketch used placeholder types (`Fact`, `Subject`, `Predicate`, a
generic `Commit`) it couldn't have known the real names for. Checked
`dmml-runtime/src/graph.rs` before writing anything: the real types are
`Commit { consumes: Vec<ConsumeRef>, produces: String, .. }`,
`ConsumeRef::{Strong(StrongRef), Fact(FactRef)}`, `FactRef { commit:
StrongRef, subject: String, predicate: String, object: Option<String>
}` — all `dmml-runtime`-local, distinct from `dmml::lower`'s own
similarly-named types in the ontology crate. Reused these directly
rather than inventing parallel ones. Also dropped GLM's `Cid`/
`Namespace` newtype wrappers in favor of plain `String`, matching this
codebase's own established convention (`StrongRef.uri`/`.cid` are
already bare `String`s everywhere) rather than introducing a wrapper
type nothing else here uses. Chose `-> impl Future<..> + Send` over
`async fn` in the trait after `cargo build` itself flagged `async fn`
in a public trait as a real, worth-fixing lint (no way to name the
`Send` bound) — confirmed by building, not assumed from general Rust
knowledge. No `async-trait` dependency: nothing in this workspace holds
a `Box<dyn Substrate>`.

## The review

Dispatched `deepseek/deepseek-v4-flash-0731` (Reviewer role) against the
real diff with the real surrounding types, per the standing Coder/
Reviewer protocol. Two real findings, reviewed point-by-point rather
than applied wholesale:

1. **Real and fixed**: `SwapOutcome::RootMoved { observed: Option<String>
   }` allowed a degenerate `expected: None, observed: None` case that
   the doc comment's own "None means empty" claim couldn't actually
   distinguish from a no-op success. Root cause once traced through: a
   rejected swap means something else committed first, so the root can
   only *advance* from what the caller expected, never regress to
   empty — `observed` should never legitimately be `None` in a real
   rejection. Fixed by making it a bare `String` and stating the
   invariant explicitly in the doc comment.
2. **Half right, half disputed**: the reviewer flagged `resolve_fact`
   living on the base `Substrate` trait as inconsistent with the
   module's own "only what both backends genuinely share" claim, since
   only `AppendSubstrate` callers are required to call it before a
   write. Agreed the doc comment overstated that exclusivity and fixed
   the wording — but disagreed with the suggested structural fix (move
   it to `AppendSubstrate` only): a `CasSubstrate`-hosted world still
   has readers materializing state or validating incoming citations who
   need exactly this same read, regardless of which write capability
   the substrate has. Kept the placement, fixed the doc.

Three other findings the reviewer raised and then correctly talked
itself out of within its own response (checked and confirmed each was
actually resolved by re-reading the trait, not just taking its
self-correction on faith).

## Proving it's actually implementable, not just plausible

`ARCHITECTURE.md` already named an in-memory mock `Substrate` as a real,
not-yet-built next step for `dmml-substrate-kit`. Built
`MockAppendSubstrate` (the iroh shape — harder of the two, since
detection is the caller's job) with four tests exercising the design's
own central claims directly, not just checking it compiles:

- a bare `produces` never retracts anything (`resolve_fact` stays
  `Live`);
- **the central one**: two commits, unaware of each other, both
  `consumes`-citing the identical prior fact — `resolve_fact` reports
  `Retracted { by: [cid_a, cid_b] }`, the real, checkable conflict
  signature the whole design turns on;
- writes are genuinely author-partitioned (`commits_by` returns exactly
  one author's own commits, in order, never another's);
- `assertions` returns every independent production for a `(subject,
  predicate)` pair, not a pre-folded current value, matching
  `pantheon.rs`'s own established multiplicity finding.

Made `dmml_runtime::graph::parse_nquads` public along the way (was
crate-private) — the mock's `assertions()` needs the exact same
`Commit.produces` N-Quads parsing `apply_commit` already does, and any
real adapter will need it too; reusing it beat duplicating it.

All four workspace crates build clean, zero new clippy warnings, full
existing test suite (99+ `dmml`/`dmml-runtime` lib tests, all example
files, all `dmml-substrate-kit` integration tests) still passes
alongside the four new ones.

## What's still open

A concrete `CasSubstrate` implementation against a live PDS (the mock
here only covers `AppendSubstrate`), and wiring an actual checkpoint
loop (build → `resolve_fact` → append-or-dispute → eventually
check-and-write to a `CasSubstrate`) on top of these traits — real,
scoped follow-on work, not blocked on anything further to design.
