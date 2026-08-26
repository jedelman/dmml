# Live deployment architecture: atproto cold, iroh hot — decided (2026-08-26)

Jason's question: what would it take to actually run this live, atproto
as cold path and iroh as hot, rather than the local-only work the
session had otherwise been doing all day. Real decisions came fast, in
order:

**1. Client split: browser + Worker are atproto-only; CLI and Android
are the only clients that ever touch iroh.** This resolves the one hard
platform wall the project had already proven firsthand (`iroh-blobs`
doesn't compile for `wasm32-unknown-unknown` — real `mio` UDP errors,
confirmed directly) by construction: iroh simply never needs to compile
to wasm32, because nothing that runs in a browser or the Worker ever
links it. The Worker's current live production deploy doesn't change at
all. Bonus, not planned for: the Android iroh spike already has
`libiroh_ffi.so` built and running on real device targets, so the
native side of this split has more prior art than the browser side ever
could.

**2. CLI/Android are concurrent, not single-writer — "it's a steering
device, not an authority."** My first pass at this (see the prior
session turn) recommended single-writer-per-session as the safer v1,
treating iroh-docs' lack of a native compare-and-swap as an unsolved
distributed-systems problem the project's own spike had stopped short
of answering. Corrected directly: that was importing the wrong mental
model. iroh-docs keys entries by `(namespace, author, key)`, so
concurrent writers from different authors never collide at the
storage/sync layer at all — iroh's own range-based set-reconciliation
already merges author-partitioned writes across peers with no CAS
needed anywhere in that path. "iroh's solved the distributed systems
problem" was the right correction; I'd been holding onto atproto's
CAS-shaped mental model past the point it applied.

**3. The real question was never sync, it's semantic reconciliation —
and DMML already has the primitive for it.** Since CLI/Android also
read atproto continuously (not just write to iroh), they're constantly
rebasing against real cold state, so divergence stays small under
normal operation; a real merge surface only opens up after genuine
extended offline work. And per Jason's framing — "we can always flag
merge conflicts, but DMML handles those natively" — the grammar already
carries everything needed: a bare `produces` (no `consumes`) never
overwrites anything, confirmed directly by `pantheon.rs`'s own
Helios/Selene/Eos coexisting with zero conflict; the *only* destructive
operation in the whole grammar is a commit's own `consumes`, which is
what actually retracts a `(subject, predicate)` key when it resolves.
So the one and only real conflict shape is two commits, unaware of each
other, each `consumes`-citing the identical prior fact as their base.
`FactRef` already carries that exact citation — a three-way-merge base
pointer that needed no new field. And the resolution mechanism was
already built and proven *earlier today*, for an unrelated reason: the
`disputes` pattern from `benjamin_rival_reading.rs` and
`autoregressive_critique.rs`'s cycles 4/6 (which spontaneously
recombined on a prior *dispute* rather than a first-order claim,
unprompted). A detected concurrent-base conflict doesn't need a winner
picked by policy — it gets surfaced as a `disputes` commit, both sides
stay real and citable, `current_value` still resolves to something via
its existing last-write-wins-by-log-order rule.

This directly replaced a harder question the project had been carrying
since the iroh spikes (a per-consume-kind `mergeable`/`arbitrated`
policy, and an `isCanonicalLeaf()` primitive to pick a winner,
`dev-journal/2026-08-24-multi-tenant-network-dmml-iroh-substrate.md`
and `2026-08-24-iroh-docs-per-author-conflict-model.md`) with something
smaller: fork *detection* is still real, scoped work; fork
*resolution*-by-picking-a-winner turns out not to be needed at all,
because disputing is a safe, uniform default for every predicate. A
future per-predicate auto-merge rule (a counter that sums instead of
disputing) is a possible later optimization, not a v1 requirement.

Wrote all of this into `ARCHITECTURE.md`'s new "Live deployment shape"
section, and correspondingly narrowed "Open design work": `Substrate`'s
two implementations are now honestly different (atproto keeps its real
CAS; iroh needs none, just the conflict check as an app-level step
before a checkpoint commit), and the only real remaining unknown in that
path is the conflict check's actual query shape against atproto's
retraction history — small, scoped, and the natural next real build,
not another open research question.

Not yet decided, flagged as still open in `ARCHITECTURE.md`: the exact
DID <-> iroh `NamespaceSecret` binding a checkpointing client needs so
its checkpoint commits land under the same sovereign identity its own
atproto reads already use.
