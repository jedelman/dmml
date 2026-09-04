# Checkpoint + atproto-sync hardening for real network unreliability (jedelman/dmml#7)

Prompted directly by Jason hitting a real 0.1Mbps, heavy-packet-loss link
and wanting written-world's sync to feel tight despite it, not by a
speculative "what could go wrong" pass. Found the real gaps by reading
the actual checkpoint/atproto code, not by assuming what a "hardening"
task usually covers.

## What was actually broken

**Checkpoint rehydration silently broke retract.** `DMML.Checkpoint`
reset every fact's `StrongRef` provenance to `Nothing` on rehydrate
(documented, but treated as a cosmetic limit); `DMML.Fire.fireTransition`
genuinely refuses to retract a fact with no real provenance to cite.
Combined: any player who ever rehydrated local state from a checkpoint
instead of full-replaying from genesis permanently lost the ability to
retract anything that predated that checkpoint. Not a corner case once
checkpointing is the normal path rather than a nice-to-have.

**`atproto-broker.sh` never checkpointed at all.** Only the intra-player
`post-merge` git hook did the checkpoint-per-commit fold. The
cross-player atproto path — the one actually exposed to a bad link — had
no equivalent.

**`DMML.Atproto.runCurl` had no timeout or retry.** No
`--connect-timeout`/`--max-time`/`--retry` anywhere. On a stalled
connection this could hang indefinitely instead of failing fast.

**`AtprotoPull.pageAll` discarded all progress on one page's failure.**
`exitFailure` on any single `listRecords` error, losing every
already-fetched page from that run — a drop on page 30 of 50 meant
starting completely over.

## What changed

1. **`CheckpointFact` now carries real `(uri, cid)` citation per
   alternative** (`CheckpointAlternative`, not a bare tuple — a future
   field doesn't need every call site rewritten). `checkpointToSnapshot`
   reconstructs a real `StrongRef` with a placeholder `Span`
   (`checkpoint:<uri>#<cid>` — `Span` is just a source-location pointer,
   nothing meaningful to recover, and nothing downstream inspects it).
   **Proved, not just typechecked**: a scratch harness materialized
   `examples/value-disambiguation-demo/world-a.dmml` with real
   provenance, round-tripped it through `snapshotToCheckpoint` →
   `encodeCheckpoint` → `decodeCheckpoint` → `checkpointToSnapshot`, and
   fired `watcher.dmml`'s `dismiss` transition against the REHYDRATED
   snapshot. Real output: `OK, resolved effects: [ResolvedRetract
   "npc/watcher" (PredIdent "watches") ... (StrongRef {strongRefUri =
   "local:...", strongRefCid = "fnv1a64:...", strongRefSpan = Span
   {spanPointer = "checkpoint:local:...#fnv1a64:..."}})...]` — the
   retract succeeded and cited the real, rehydrated ref.
2. **`atproto-broker.sh` checkpoints too now**, ported directly from
   `post-merge`'s own pattern: look up a checkpoint keyed by the
   PRE-incorporation `commits/` tree; if found, fold only this run's
   incorporated peer files into it; if not found (genesis, or a prior
   attempt that failed), fold everything at HEAD instead. That fallback
   is what makes it self-healing — a missing checkpoint, whatever the
   reason, always triggers a full fold rather than leaving the chain
   broken, same as `post-merge` already did; no separate mechanism was
   needed for this once the same pattern was ported faithfully. Only
   active when `commits-dir` is literally `commits` (checkpoints are
   keyed by git's own tree hash for that exact tracked path) and
   `DMML_CHECKPOINT_REBUILD` is set — both optional, so existing callers
   are unaffected.
   **Proved live**, against `claude.jason-edelman.org`'s real PDS, in a
   throwaway repo: first run (bootstrap, no parent checkpoint) correctly
   folded both the local genesis file and the newly-pulled peer file
   (`checkpoint-rebuild: ... 2 folded as commits ... over parent none`);
   a second run with nothing new correctly no-op'd (no spurious
   checkpoint commit); git plumbing directly confirms the resulting HEAD
   commit's own tree already satisfies the "parent checkpoint found"
   lookup for whatever round comes next.
3. **`runCurl` gained real network flags**: `--connect-timeout 15
   --max-time 60 --retry 3 --retry-delay 2 --retry-connrefused`.
   Verified live against this session's own real (also flaky) sandbox
   network: a `resolveHandle` call hit a genuine `Connection reset by
   peer`, curl's own retry logic engaged (~11s total, consistent with
   the configured retry/delay), and the process still failed cleanly
   with a real error rather than hanging. **Honestly disclosed**: this
   sandbox routes outbound HTTPS through its own proxy, so a true
   "connection stalls forever" scenario couldn't be independently
   simulated here (a raw connect to a black-holed IP fails fast at the
   TLS layer instead, proxy-side) — the connect-timeout/max-time upper
   bounds are typechecked and structurally correct, but that specific
   stall case is unverified beyond code review.
4. **`AtprotoPull.pageAll` retries a failing page** (bounded, 3
   attempts) before giving up on that page specifically, and on final
   failure returns whatever was already fetched instead of exiting
   empty-handed. Verified: the existing "total failure" path (bad host)
   still exits cleanly with real retry messages; the specific "page 1
   succeeds, page 2 fails, page 1's records survive" branch is
   typechecked and code-reviewed but not independently fault-injected
   against a live multi-page pull — no controllable way to force a
   mid-pagination failure against a real PDS from this sandbox. Real,
   disclosed limit, not claimed as proven.

## Not touched, real and disclosed

Checkpoint pruning (unbounded `checkpoints/` growth) and
machine-definition checkpointing remain exactly as scoped in
`jedelman/dmml#1` — unrelated to network reliability, not folded into
this pass.
