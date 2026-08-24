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

## Open design work (named, not designed here)

- **`Substrate`'s real method signatures.** The write-with-admission-
  gate contract, informed by two independently-verified real findings:
  atproto's `swapCommit` gives a genuine compare-and-swap (written-
  world's `server/src/atproto/commit_write.rs`, verified live against a
  real PDS); iroh-docs gives none (`spikes/iroh-chain-integrity/
  gated_chain_append.rs`) — the gate has to be application code there,
  and real multi-writer fork resolution needs a per-consume-kind
  `mergeable`/`arbitrated` policy, not one uniform rule
  (`dev-journal/2026-08-24-multi-tenant-network-dmml-iroh-substrate.md`).
- **`isCanonicalLeaf()`** — the real primitive `mergeable`/`arbitrated`
  resolution needs, replacing raw existence-checking once concurrent
  forks stop being rare (they currently aren't, in written-world,
  because petition resolution is serialized through one Durable
  Object). iroh-docs gives real, free, nondestructive multi-writer
  storage (entries are keyed by `(namespace, author, key)`, so
  different authors never collide) but does **not** give leaf-selection
  for free — `Query::single_latest_per_key()` is a naive raw-timestamp
  read projection, not a semantically-aware fork resolution, and using
  it as `isCanonicalLeaf()` would be a regression
  (`dev-journal/2026-08-24-iroh-docs-per-author-conflict-model.md`).
- **Cross-substrate identity.** A sovereignty root has to be
  represented across an atproto DID and an iroh `NamespaceSecret`
  without either shape leaking into the trait.
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
independently proven out:

- A write path that either provides a real compare-and-swap (atproto's
  `swapCommit`) or, if the underlying store doesn't (iroh-docs doesn't),
  implements the admission gate as application code and declares a
  fork-resolution policy (`mergeable` or `arbitrated`) per consume-kind.
- A CID/identity representation `dmml-runtime`'s `apply_commit` can
  treat as an opaque, comparable string — whatever the underlying hash
  scheme actually is, wrapped to a common shape (a `dmml-substrate-kit`
  module's job, not the adapter's own).
- A sovereignty root (one owner, one namespace/repo) — "only ever check
  one repo" is a real, load-bearing simplification both adapters
  already rely on, not incidental.
