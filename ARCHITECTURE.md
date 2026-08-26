# Architecture

## Why 3 crates, and why the boundary sits where it does

This repo exists because DMML's ontology and its runtime materializer
turned out to already be *mostly* substrate-blind by accident: written-
world's `engine::graph::WorldGraph::apply_commit` never touches atproto
or iroh directly — it compares opaque `{uri, cid}` strings, nothing
more. The one real exception was `dmml::identity::compute_cid`, which
baked in atproto's exact `CIDv1(dag-cbor, sha2-256)` wire shape and a
hardcoded record NSID. Extracting this repo meant making that
exception explicit rather than letting it quietly define what "the
ontology" actually depends on.

- **`dmml`** — the ontology itself: grammar, AST, parser, interpreter
  (`machine.rs`), validation, view. Handles `StrongRef`/`ConsumeRef` as
  opaque `{uri, cid: String}` pairs and never computes or verifies a
  hash. If you're asking "what does DMML *mean*," this is the crate.

- **`dmml-runtime`** — the materializer: an oxigraph-backed
  `WorldGraph`, `apply_commit`, percepts, commune/demiurge. This is
  where a `Commit` actually lands and where written-world's own #53
  fix (checking a `consumes` reference's `cid` against every `foreignCid`
  fact ever recorded for that `uri` — existence, not currency) lives.
  It also owns `substrate::Substrate`, the trait boundary a concrete
  backend has to satisfy — currently a named stub, not a finished
  design (see "Open design work" below).

- **`dmml-substrate-kit`** — genuinely shared, substrate-*specific*
  tooling that more than one concrete adapter would otherwise
  duplicate: today, `atproto_cid` (the extracted `CIDv1(dag-cbor,
  sha2-256)` strategy, byte-compatible with a real atproto PDS for
  predicates, not yet fully for subjects/objects — see that module's
  own doc comment for the precise, still-open gap). An `iroh_cid`
  module (wrapping iroh-blobs' raw BLAKE3 as a CIDv1 under the
  registered BLAKE3 multicodec) and an in-memory mock `Substrate` for
  testing `dmml-runtime` with zero network dependencies are named next
  steps, not yet built.

## What's deliberately NOT in this repo

Concrete atproto or iroh network wiring. Two real, verified reasons:

1. **A hard platform wall, proven firsthand.** `iroh-blobs` does not
   compile for `wasm32-unknown-unknown` — real compiler errors deep in
   `mio`'s UDP socket code (`no method named 'register' found for
   struct IoSource<std::net::UdpSocket>`, a mismatched-types error on
   raw-fd conversion), confirmed by actually adding the dependency to a
   wasm32 target and running `cargo check`, not assumed. Any crate that
   needs to compile to wasm32 for a Cloudflare Workers consumer (like
   written-world's `server/`) cannot also carry a concrete iroh
   dependency, even behind a feature flag someone forgets to check.
2. **A real consistency argument.** written-world's `spikes/iroh-chain-
   integrity/`'s own `Cargo.toml` already isolates itself from the
   shipped build for exactly this reason, one level down. Keeping
   concrete substrate glue application-side (written-world's
   `server/src/atproto/` for the atproto path; a hypothetical iroh-first
   app owning its own adapter) is the same discipline, one level up.

## Live deployment shape: atproto cold, iroh hot (decided 2026-08-26)

The client/substrate split is decided, not open: **browser and the
Cloudflare Worker are atproto-only; CLI and Android are the only
clients that ever touch iroh.** This resolves the platform wall above
by construction rather than by workaround — iroh never needs to compile
to `wasm32` at all, because nothing that runs in a browser or a Worker
ever links it. The Worker's current live production deploy (written-
world's `written-world`/`written-world-dev`) doesn't change shape.

CLI and Android are **steering devices, not an authority** — they hold
no canonical state of their own. Each also reads atproto directly (not
just iroh), so a hot-path client is continuously rebasing against the
real cold record; the only time it accumulates meaningful divergence
from atproto is genuine extended offline work, not ordinary operation.
This, not a sync-layer limitation, is what makes checkpoints small and
frequent the right default rather than rare and batched.

**The sync layer needs no new design.** iroh-docs keys entries by
`(namespace, author, key)`, so CLI and Android writing concurrently
never collide at the storage/sync layer at all — each writer has its
own partition, and iroh's own range-based set-reconciliation protocol
merges those partitions across peers with no compare-and-swap required
anywhere in that path. The earlier framing of this as an unsolved
distributed-systems problem was importing the wrong model (one
CAS-guarded mutable head, atproto's shape) onto a grammar that never
needed one — DMML has no primitive for editing a fact in place at all
(`README.md`; Section 1 of `papers/desiring-production-ontology/
DRAFT.md`), so "concurrent writers" was never actually a storage
problem here.

**The only genuinely destructive operation in the grammar is a
commit's own `consumes`.** A bare `produces` — no `consumes` — never
overwrites or contests anything; it only ever adds a new, independently
citable data point, exactly what `pantheon.rs`'s Helios/Selene/Eos
already demonstrate (three uncoordinated `origin` assertions for the
same node coexist with zero conflict, Checks 1-2). Retraction only
happens when a commit's `consumes` cites a specific prior `(subject,
predicate)` and gets accepted — that citation is what does the
retracting (`fact_retraction_fails_open`'s own counterpart case: when
the citation genuinely resolves, the key really is retracted). So the
one and only shape a real merge conflict can take is: **two commits,
each unaware of the other, each `consumes`-citing the identical prior
`(uri, cid, subject, predicate)` as their base, each about to retract
it.** Nothing else in the grammar needs a conflict check — this is not
a simplifying assumption, it follows from `consumes` being the
grammar's only destructive primitive.

**Detection needs no new field.** A `FactRef` already carries `{commit:
StrongRef, subject, predicate, object}` — the exact prior commit a new
fact was built on top of, i.e. already a three-way-merge base pointer.
Two commits from different authors citing the *same* base for the same
key is the entire, checkable signature of true concurrency; nothing
resembling a vector clock needs inventing.

**Resolution is `disputes`, not arbitration.** A detected concurrent-
base conflict is not blocked and does not need a winner picked by
policy — it's surfaced as a `disputes` commit, the same pattern already
proven this session (`benjamin_rival_reading.rs`'s dispute of a rival
claim; `autoregressive_critique.rs`'s cycles 4 and 6, which
spontaneously recombined on a *prior dispute* rather than a first-order
claim, unprompted). Both retracting commits stay real and independently
citable; `current_value` still resolves to *something* via its existing
last-write-wins-by-log-order rule, same as it already does for
`pantheon.rs`'s un-consumed rival asserts — the dispute is what makes
the disagreement visible and citable, not what blocks progress. This
replaces the harder question the project was previously carrying (a
per-consume-kind `mergeable`/`arbitrated` policy, and an `isCanonicalLeaf()`
primitive to pick a winner) with something smaller: fork *detection* is
still real work, but fork *resolution*-by-picking-a-winner turns out not
to be needed at all — disputing is a safe, uniform default for every
predicate. A future per-predicate auto-merge rule (e.g. a counter that
sums instead of disputing) remains a possible optimization on top of
this, not a v1 requirement.

## Open design work (named, not designed here)

- **`Substrate`'s real method signatures**, now split honestly rather
  than unified: the atproto side keeps its real, already-proven
  compare-and-swap (`swapCommit`, written-world's `server/src/atproto/
  commit_write.rs`); the iroh side needs **no CAS at all** (writes are
  author-partitioned, per above) but does need the concurrent-base
  conflict check above to run as an application-level step before a
  checkpoint commit goes out to atproto.
- **The conflict check's actual implementation.** Detecting "did
  someone else's commit already retract the `(subject, predicate)` key
  my pending commit consumes, since I last saw it" requires the
  checkpointing client to query atproto's retraction history for that
  key at checkpoint time — not yet built, and the exact query shape
  (a single `(uri, cid, subject, predicate)` existence check, per
  written-world's #53 precedent, or something broader) is real,
  scoped, comparatively small design work now.
- **Cross-substrate identity.** A sovereignty root has to be
  represented across an atproto DID and an iroh `NamespaceSecret`
  without either shape leaking into the trait — still open, and now
  more concrete: a CLI/Android checkpoint commit needs to be written
  under the same DID its own atproto reads already use, so the binding
  has to be real, not just colocated on one device.
- **Cross-DID references stay quotation, not verification** — a
  foreign-node reference materializes as a first-person, timestamped
  `Percept` (the same primitive written-world's sense-machines already
  use), never a live-verified claim about another party's current
  canonical state. Two DIDs quoting the same third party can
  permanently disagree; that's a legitimate steady state, not an
  inconsistency awaiting convergence.

## What a new substrate adapter needs to implement

Not yet real (the trait itself isn't finished — see above), but the
shape it needs to satisfy, from the two adapters already
independently proven out and the live-deployment shape above:

- A write path that either provides a real compare-and-swap (atproto's
  `swapCommit`) or, if the underlying store doesn't need one because
  writes are already author-partitioned (iroh-docs), runs the
  concurrent-base conflict check as an application-level step before
  any commit that would retract a shared key leaves the local node.
- A CID/identity representation `dmml-runtime`'s `apply_commit` can
  treat as an opaque, comparable string — whatever the underlying hash
  scheme actually is, wrapped to a common shape (a `dmml-substrate-kit`
  module's job, not the adapter's own).
- A sovereignty root (one owner, one namespace/repo) — "only ever check
  one repo" is a real, load-bearing simplification both adapters
  already rely on, not incidental.
