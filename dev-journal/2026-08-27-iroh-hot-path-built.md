# The hot path, for real: `IrohAppendSubstrate` and a live pantheon swarm (2026-08-27)

Jason: "build with glm5.3! I'd like to end up with hot and cold paths live
and an enrichment swarm - still on that pantheon idea." Cold path was
already live (the Benjamin essay, real records on `claude.jason-
edelman.org`). This session built the hot path's missing half: a real,
non-mock `AppendSubstrate` over real `iroh-docs`, proven with a real
multi-author swarm re-enacting `pantheon.rs`.

## Grounding first, dispatch second

`iroh`'s API surface is real, current, and not something to guess from
training data — the standing dispatch-methodology note already warns
this exact model class hallucinates unfamiliar call signatures. Rather
than trust memory, downloaded the actual `iroh` 1.1.0 and `iroh-docs`
0.101.0 crate sources from `static.crates.io` and read them directly:
`Docs::memory().spawn(endpoint, blobs, gossip)`, `DocsApi::{author_create,
create, open}`, `Doc::{set_bytes, get_many, get_exact}`, `Query::{all,
key_exact}`, `Entry::{author, key, content_hash}` — all confirmed against
real source, including the crate's own `examples/setup.rs`.

## The dispatch, and what it got wrong

Gave `z-ai/glm-5.3` the real `Substrate`/`AppendSubstrate` trait, the
real `Commit`/`ConsumeRef`/`FactRef` types, `mock.rs`'s exact matching
logic to mirror, and the real iroh-docs API gathered above. Two real
timeouts first: a combined two-file prompt at `reasoning: high` returned
nothing but SSE keep-alive padding after both 280s and 580s — confirmed
the route itself was healthy (a trivial prompt round-tripped in 5s), so
this was genuine reasoning time on a large, detailed spec, not an outage.
Split to one file, dropped to `reasoning: low` (the task was transcription-
with-care against exact given types, not open design — didn't need deep
reasoning), and it returned cleanly in about a minute.

**Real, worth-catching mistake**: despite being handed `mock.rs`'s exact
working code as ground truth, GLM's `assertions()` matched N-Quads
subject/predicate as `oxigraph::model::Term::NamedNode(..)` for both —
but the real type is `NamedOrBlankNode` for `subject` and a bare
`NamedNode` (never wrapped in `Term`) for `predicate`. Wouldn't have
compiled. Fixed by copying `mock.rs`'s proven match arm verbatim instead
of the generated one.

## What compiling for real actually caught

- `iroh-blobs = "0.101"` conflicts with `iroh-docs 0.101.0`'s own
  transitive requirement of `iroh-blobs = "0.103"` — the version-number
  coincidence between `iroh-docs` and `iroh-blobs` releases isn't a real
  pairing; had to check `iroh-docs`'s own `Cargo.toml` to find the real
  compatible version.
- `Doc`/`Query` live at `iroh_docs::api::Doc` / `iroh_docs::store::Query`,
  not the crate root (only `Entry`/`AuthorId` are root re-exports) —
  caught by the compiler's own "did you mean" suggestion, not guessed.
- `Stream::next()` needs the stream pinned (`tokio::pin!`) since
  `get_many`'s returned `impl Stream` isn't `Unpin` — `api.rs`'s own
  `get_one` does exactly this; missed on the first pass, caught by a real
  trait-bound compiler error, fixed by matching the crate's own pattern.
- `set_bytes`'s `key: impl Into<Bytes>` needs owned/`'static` data — a
  borrowed `cid.as_bytes()` doesn't satisfy it. Fixed with
  `cid.clone().into_bytes()`.

## The real hang, and the real fix

First working build used `Endpoint::builder(presets::N0).relay_mode(
RelayMode::Disabled).bind()` — compiled clean, then hung indefinitely on
`cargo run` (confirmed via a hard timeout, not assumed). N0's preset
still wires up real internet discovery/relay services even with the
relay itself disabled afterward, and this sandbox's proxy doesn't carry
whatever those services need — the same class of hang as this session's
earlier Chromium-in-sandbox proxy issue, different subsystem.

Fixed by dropping the preset entirely: `Builder::empty()` (no address
lookup services, `RelayMode::Disabled`, by its own doc comment) plus an
explicit `crypto_provider` (the one thing `empty()` doesn't set, which
`bind()` requires) via `rustls::crypto::ring::default_provider()` —
confirmed `ring` is iroh's own default TLS feature, added `rustls`
directly rather than relying on the transitive dependency. Ran instantly
once switched.

## The real swarm

`dmml-substrate-kit/examples/pantheon_swarm.rs`, `cargo run --example
pantheon_swarm`:

1. `helios`/`selene`/`eos` — three real `AuthorId`s, three independent
   `produces`-only commits asserting `sky/1`'s origin. Real read-back via
   `assertions()` shows all three coexist — the original `pantheon.rs`
   finding, now over genuine content-addressed storage, not an in-memory
   `Vec`.
2. `nyx` and `pantheon_council` — two more real authors, each
   independently `consumes`-citing `helios`'s real CID to produce a rival
   synthesis (`duskweave` vs `starforge`), unaware of each other. Real
   `resolve_fact` call returns `Retracted { by: [both real CIDs] }` —
   the actual conflict signature, detected against real storage.
3. `pantheon_council` appends a real `disputes` commit citing both
   rivals, picking neither — `<x:sky/1> <x:disputedOrigin>
   "duskweave-vs-starforge, unresolved">`.
4. Final listing: 6 real entries, 6 real distinct author IDs, every key a
   real BLAKE3-of-JSON CID.

Deliberately scoped to one process — every "god" is a distinct author
writing the same local `Doc`, not a separate network node. iroh-docs'
`(namespace, author, key)` partitioning is what makes that a faithful
proof of the concurrent-writer story regardless of process count; real
multi-node sync (`doc.share()`/`api.import()` between separate
`Endpoint`s) is the honest next step, named but not built here.

## What's still open

A concrete `CasSubstrate` (cold path is already live for real writes —
the Benjamin essay, 46 real records on `claude.jason-edelman.org` — but
not yet wrapped in the trait), and the checkpoint loop bridging hot →
cold (build → `resolve_fact` → append-or-dispute → eventually
check-and-write via `CasSubstrate`). Both scoped, neither designed here.
