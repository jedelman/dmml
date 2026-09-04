# Real consumes citation-integrity checking (jedelman/dmml#6)

Closes the gap found while rewriting `dmml-agent-nucleus/GRAMMAR.md` to
make `dmml-hs` canonical: `DMML.Materialize.applyConsume` matched a
`consumes` citation purely on `(subject, predicate[, object])` and never
checked its `StrongRef` (`uri`/`cid`) against anything -- a citation
naming a `cid` nobody ever actually saw was accepted exactly the same as
a real one. The retired Rust crate's `graph.rs` had *something*: checked
a citation's `cid` against what it had actually recorded as observed for
that `uri`.

## What "observed" means here, for two real cases

1. **A file actually present in the batch being checked.** Its own real
   identity -- `DMML.LocalIdentity.localFileRef`, recomputed from its
   own exact bytes -- is authoritative. Confirmed this is exactly how a
   citation gets INTO a real commit in the first place:
   `DMML.Fire.renderFiredCommit` writes `fact <uri>#<cid>` straight from
   a `StrongRef` built by `localFileRef` on the same `--world`/
   `--machine` files given to `fire-transition`. So checking a citation
   against the actual file it names, when that file is right there, is
   a REAL check -- not first-citation-wins, because the truth is
   independently known.
2. **A uri for a file not in the batch** (a citation to another repo's
   commit, or a peer's commit not materialized here). Nothing to
   independently check it against, so the first citation seen
   establishes what "the" cid for that uri is taken to be; a later
   citation of the same uri under a different cid is rejected as
   inconsistent. Same real, if weak, first-writer-trust check the
   retired Rust crate had -- `written-world/SPEC.md` already discloses
   its limit (a writer can still poison a node's first-seen cid record,
   no verification against real substrate content) -- not solved here,
   same as it was never solved there.

## What's built

`DMML.CitationIntegrity` (new module): `CidLedger`, `seedObserved` (feed
in real, independently-known identities before checking), `checkCommit`/
`checkCommits` (walk every `consumes` citation -- both `ConsumeStrong`
and `ConsumeFact`'s own `factConsumeCommit` -- threading and checking
against the ledger, in order, stopping at the first mismatch).
`check-citations` CLI: reads a batch of files, seeds the ledger from
each file's own real `localFileRef`, then checks every commit's
citations, exit 0 or 1 with the exact mismatch.

Deliberately NOT scoped here: `DMML.Ast.ConsumeStrong` (a whole-record
strongRef -- a Bridge half, a Pentacle grant, not a specific fact) still
isn't *applied* by `applyConsume` at all, a real, separate, disclosed
gap. Its citation IS still checked for internal consistency here, same
as a `ConsumeFact`'s -- whether or not anything downstream acts on it
yet.

## Real verification, not just unit-level

Three real scenarios, not hypothetical:

1. **Real citation against the real file it cites** (`fire-transition`'s
   own live output, `examples/chained-retract-demo/`): accepted.
2. **The same real output, cid manually tampered** (`sed`'d to a
   different hex string): rejected, with the exact uri, the recorded
   cid, and the tampered one.
3. **External-uri fallback** (`examples/citation-integrity-demo/`): a
   citation to a uri with no local file is accepted the first time
   (nothing to check against), and a SECOND citation of the same uri
   under a different cid, later in the same batch, is caught.

All three wired into `.github/workflows/dmml-hs-ci.yml` and re-run
locally against the exact CI commands before pushing (this sandbox has
no path to trigger real Actions runs, same disclosed limit as the rest
of this project's CI work).

## Real mistake made and caught while building this, not hidden

First test invocation of `fire-transition` in this session
(`fire-transition keeper.dmml witnessEruption npc/keeper --world
world.dmml ...`) passed a node ref (`npc/keeper`) as the CLI's `<verb>`
argument instead of an actual verb, by copying an old invocation without
re-checking `fire-transition`'s own usage line. This produced `commit
npc/keeper` -- which doesn't parse (`commit <verb>` wants a bare
identifier, not a `/`-containing node ref) -- and looked, briefly, like
a real regression in `renderFiredCommit` itself. It wasn't: re-reading
`FireTransition.hs`'s own `parseArgs` confirmed argv position 3 really is
the verb, and re-running with a real verb (`witnesses`) produced a
normal, valid commit. Worth naming because the earlier session (before
this one) made almost the identical mistake for a different reason
(writing DMML.Atproto's own test content with an invalid `commit
<name>` and only catching it when `validate-commit` rejected it for
real) -- the grammar's own `commit <verb>` rule is evidently easy to get
wrong from muscle memory, twice, by the same session.

## Status

`jedelman/dmml#6` closed by this commit. `jedelman/dmml#4` was already
closed earlier today (the real strong-ref provenance this citation
checking depends on). `written-world#138`'s Phase 1 checklist updated.
