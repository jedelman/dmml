# Real EffectRetract firing: closing jedelman/dmml#4

Jason: "yep, let's fix retract." Closes the gap Phase 3 disclosed
(`dev-journal/2026-09-03-phase-2-3-effect-generalization-and-firing.md`)
and that the same-day cascade demo then made concretely visible
(`dev-journal/2026-09-04-sense-machines-and-caller-driven-cascades.md`):
`DMML.Fire` could resolve an `EffectAssert` to a real commit but always
refused an `EffectRetract`, because DMML's real commit grammar only
retracts a fact via a `consumes` block naming the SPECIFIC prior commit
(`uri#cid`) that asserted it, and a `WorldSnapshot` built the ordinary
way never carried one.

## The fix, in order

1. **`DMML.Materialize.Alternatives`** now carries an optional real
   `StrongRef` per live alternative, not just a `(label, value)` pair.
   Fully additive: `alternativeValues` (the accessor every existing call
   site reads through -- `DMML.Guard`, `DMML.Governance`,
   `DMML.Retroconsistency`, `renderSnapshot`, every `app/*Demo.hs`) keeps
   its exact old type and behavior, dropping the new provenance field.
   `applyCommit`/`applyCommits` (the label-only, no-real-provenance path
   every existing caller uses) are unchanged in behavior -- internally
   they now call a shared `applyCommitWithRef` with `Nothing` for the
   ref, byte-for-byte the same result as before.

2. **New parallel entry point**, mirroring the real Rust crate's own
   `IdentifiedCommit { uri, cid, commit }` shape (`dmml::interpret`) --
   `DMML.Materialize.applyIdentifiedCommit`/`applyIdentifiedCommits` take
   a real `StrongRef` per commit and tag every fact it asserts with it.
   New `currentValueWithProvenance` reads it back out for `DMML.Fire`.

3. **`DMML.LocalIdentity`** (new module): `dmml-hs`'s toolchain has no
   SHA-256 library available and no Hackage access to fetch one (real
   constraint, checked via `ghc-pkg list` before assuming otherwise --
   this repo's own README already disclosed "no cabal update/Hackage
   access needed" as a deliberate constraint). Rather than label
   something `sha256:...` that isn't actually SHA-256 -- a false
   compatibility claim, worse than admitting the gap -- this computes a
   real, standard FNV-1a (64-bit) fingerprint over a file's own exact
   bytes and labels it honestly: `local:<path>` / `fnv1a64:<hex>`. Real
   content addressing (same bytes always produce the same `cid`,
   different bytes never collide in practice), explicitly NOT
   interoperable with a real atproto-issued CID for the same content --
   the module's own doc comment says so, so nothing downstream can
   mistake this for the production identity scheme.

4. **`DMML.Fire`** now resolves a retract effect by looking up
   `currentValueWithProvenance` for its (subject, predicate):
   - zero live alternatives → `FireRetractNoSuchFact` (nothing to
     retract)
   - exactly one, with real provenance → builds a `consumes`/`fact`
     citation, real
   - exactly one, but materialized without real provenance (plain
     `applyCommit`) → `FireRetractNoProvenance`, refuses rather than
     fabricate a citation
   - more than one live alternative → `FireRetractAmbiguous`, refuses.
     Real reason, not overcaution: `DMML.Materialize`'s own `consumes`
     application (`applyConsume`) deletes EVERY live alternative for a
     (subject, predicate) key unconditionally, regardless of which
     `uri#cid` the entry names -- so citing just one of several live
     alternatives' provenance while the applied commit would silently
     delete all of them (including ones never cited) would misrepresent
     what's actually being consumed.

   `ResolvedFact` (asserts) and the new `ResolvedRetract` (subject,
   predicate, cited `StrongRef`) are both cases of a new `ResolvedEffect`
   sum type, so `renderFiredCommit` renders asserts and retracts in the
   transition's own declared order, with every retract folded into one
   trailing `consumes` block (real Surface grammar, verified by parsing
   it back before relying on it -- `consumes` / `fact <uri>#<cid>` /
   `subject . predicate`, using megaparsec's real `indentBlock`, not a
   fixed column count).

5. **`app/FireTransition.hs`**: every `--world` file is now materialized
   via `applyIdentifiedCommits`, its `StrongRef` computed from the file's
   own bytes via `DMML.LocalIdentity.localFileRef`. No new CLI flag
   needed -- provenance is automatic and free for any world file the
   caller already has to name.

## Real verification, not just "it compiles"

Whole demo suite (`GuardDemo`, `GovernanceDemo`, `RetroconsistencyDemo`,
`RetroChainDemo`, `RetroGateDemo`) reruns with identical output, no
regressions. All 12 real machine files still parse.

`examples/retract-demo/`: `examples/shrine.dmml`'s real `awaken`
transition (`assert awakened`, `retract stirring` -- authored back in
the original spike, long before this fix existed to make it fireable) is
the actual test case, not a synthetic one built to be easy:

```
$ fire-transition examples/shrine.dmml awaken awakens \
    --world examples/retract-demo/world-clean.dmml --param witness=npc/keeper

commit awakens
  declare relation state
  shrine/threshold `state` awakened
  consumes
    fact local:examples/retract-demo/world-clean.dmml#fnv1a64:7b932dc7eed75ef0
      shrine/threshold . state
```

Passes real `validate-commit`/`check-declared`. Re-materializing
`world-clean.dmml` + this fired commit together and rendering the
snapshot confirms `state` is genuinely gone from the fact store, not just
described as retracted -- `shrine/threshold . witnessedBy = npc/keeper`
is the only fact left, `state` doesn't even appear.

Both refusal paths checked too, not just the happy path:
- **Ambiguous**: seeded `state` with two independently-committed live
  alternatives (`dormant` and `stirring`, two separate world files) --
  `fire-transition` correctly refuses (`FireRetractAmbiguous`) rather
  than pick one.
- **No provenance**: a snapshot built the ordinary `applyCommits` way
  (no real `StrongRef`s at all) correctly refuses
  (`FireRetractNoProvenance`) rather than fabricate a citation.

**The cascade demo's original bug is now fixed at its actual root**, not
just worked around: `furnace.dmml`/`anvil.dmml` (`dev-journal/2026-09-
04-sense-machines-and-caller-driven-cascades.md`) now `retract idle`
alongside `assert <newstate>`. Rerun manually round-by-round: round 1
fires both transitions for real (each producing a real `consumes` block
retracting its own `idle`), round 2 now REFUSES both with "transition's
guards do not currently hold" -- the from-state guard genuinely goes
false, not just "technically legal but nothing new" the way the
hash-dedup workaround had to detect before. `run.sh` was updated to keep
the hash-dedup as defense-in-depth (protects a caller against a machine
author who forgets to write a real retract) rather than remove it, but
this demo no longer needs it to reach its own fixpoint -- confirmed by
rerunning it end to end: one real cascade round, clean stop on genuine
guard refusal in round 2.

## What's still open

- `DMML.Checkpoint`'s round-trip still drops provenance entirely (never
  carried a real `StrongRef` before this fix, still doesn't -- a real,
  disclosed, pre-existing limitation, not new: a snapshot rebuilt from a
  checkpoint file can't retract anything until the checkpoint format
  itself grows a `cid` field, not yet needed by anything that reads one
  today).
- `DMML.Governance.collapseToOne` always collapses to a provenance-free
  (`Nothing`) alternative -- an arbitrated OUTCOME has no single real
  citation that would honestly describe it, so a governed-then-collapsed
  fact can't be retracted via `DMML.Fire` either, without a further
  design pass on what "the provenance of an arbitration outcome" should
  even mean.
- `DMML.LocalIdentity`'s FNV-1a fingerprint is a real, deterministic
  local identity, not a real atproto CID -- when `dmml-hs` (or its
  eventual real-substrate successor) needs interop with actual PDS-issued
  CIDs, this module gets replaced, not extended; the naming (`fnv1a64:`
  never `sha256:`) exists specifically so nothing downstream mistakes one
  for the other in the meantime.
