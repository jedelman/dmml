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

## Cross-substrate identity: DID stable, endpoint rotates, binding is a record

Two different rotation problems were sitting under one label
("identity binding") until Jason separated them: a DID's *service
endpoint* (which PDS currently hosts it) rotates — a user can migrate
PDS providers — while the DID itself is the stable anchor. Conflating
"bind an iroh author key to a DID" with "know where that DID's PDS
currently lives" would have meant solving both at once. They don't need
to be solved together, and one of them is already solved.

**Endpoint resolution is already real, live, and reusable as-is.**
written-world's `server/src/atproto/identity.rs` doesn't cache a PDS
host anywhere — `WwDidResolver = CommonDidResolver<WorkersHttpClient>`
resolves a DID's current `serviceEndpoint` fresh, via `atrium-identity`
(not hand-rolled), confirmed generic over its own HTTP transport
(`CommonDidResolver<T: HttpClient>`). The only Workers-specific part is
`WorkersHttpClient` — a thin transport shim, not resolution logic. A
native CLI/Android client needs the exact same `atrium-identity`
dependency and its own native `HttpClient` impl (`reqwest` or
equivalent) plugged into the identical `CommonDidResolver<T>` — reusing
the same audited resolution logic the Worker already runs in
production, not reimplementing DID/endpoint rotation a second time.
This was previously named as an open question; it isn't one anymore,
it's a known, scoped implementation task (write one transport shim).

**The DID↔iroh binding itself is a new record, not a new mechanism.**
Since endpoint rotation is already handled by resolving fresh at use
time, the binding fact doesn't need to mention an endpoint at all — it
only needs to say "this iroh author key belongs to this DID," published
as an ordinary record in the player's own PDS repo, written through the
same `swapCommit`-gated path every other commit already uses
(`commit_write.rs`). Because it's a record *in that DID's own repo*,
proof of authorship is the same proof atproto already relies on
everywhere (only the DID's controller can write to that DID's repo) —
no new signature scheme, no new trust root. And because repo migration
is already atproto's job, the binding record moves with the DID under a
PDS migration for free, the same as every other record in that repo.

The binding is between the DID and an iroh **Author** key specifically
(`AuthorId`/`AuthorSecret` in iroh-docs' own vocabulary), not the
**Namespace** — the namespace is the shared world/document identity;
the author is the per-writer key `(namespace, author, key)` entries are
actually keyed by, and it's the author key a specific device's writes
need to be attributable to.

**Rotation is not a new mechanism either.** A lost device, a new
device, or a routine key rotation is just another commit: `consumes`
the old binding fact, `produces` the new one — the exact same
consume-old/produce-new discipline every other fact change in this
whole grammar already uses. No bespoke revocation or rotation primitive
needed; the binding fact is ordinary DMML content, checkable the same
way everything else here is.

**Android's auth path is decided: OAuth.** Not the app-password
alternative — a revocable, no-embedded-secret flow is the right shape
for a distributed consumer app, and the reuse story is the same shape
as DID resolution above: `WwOAuthClient = atrium_oauth::OAuthClient<
WwStateStore, WwSessionStore, WwDidResolver, WwHandleResolver,
WorkersHttpClient>` is generic over all five of those type parameters,
so the actual OAuth protocol machinery (PAR, PKCE, DPoP, token
exchange) is the same audited `atrium-oauth` code already proven live
in the Worker, not a reimplementation — Android needs its own
`HttpClient` (shared with the DID-resolution shim above) and its own
`StateStore`/`SessionStore`, backed by Android's own secure storage
(Keystore-backed), not the Worker's KV-backed one.

**One real difference, found by checking the actual config, not
assumed reusable as-is**: this deploy's `atproto_client_metadata`
configures a **confidential client** —
`token_endpoint_auth_method: PrivateKeyJwt`, authenticated to the PDS's
token endpoint with a server-held private signing key (`app_jwk`).
That specific configuration cannot be copied into Android — a "private"
key embedded in a distributed, decompilable APK isn't private, and
shipping it would defeat the whole point of `private_key_jwt` client
authentication. Android needs atproto's **public-client** OAuth path
instead (PKCE only, no client assertion) — the same `atrium_oauth::
OAuthClient` machinery, a different `AtprotoClientMetadata`/`AuthMethod`
configuration. Whether `atrium-oauth` 0.1.7 exposes that public-client
mode directly is unverified here — a real first check for whoever
scopes this, not assumed.

A second, genuinely new piece of infrastructure this needs: Android's
OAuth redirect can't reuse `{public_url}/oauth/callback` (that's the
Worker's own server-side callback handler) — it needs a verified HTTPS
App Link that the OS routes back into the app after the system browser
completes authorization (RFC 8252's native-app pattern), which means
hosting a `.well-known/assetlinks.json` for Digital Asset Links
verification. The `client_id` itself can still point at the existing
`{public_url}/client-metadata.json` — that document is static discovery
metadata, not part of the live flow, and doesn't need to change shape
for a public client beyond its `token_endpoint_auth_method` field.

**CLI's auth path is still genuinely open** — the OAuth flow above
(even in its public-client form) assumes a system browser and a
routable redirect, neither obviously available to every CLI context;
an app-password session (`com.atproto.server.createSession`) is simpler
but a different trust and revocation model, and a loopback-HTTP-server
pattern (open a local port, launch the system browser, catch the
redirect there — the same shape `gh auth login` and similar CLIs
already use) is a third real option this doc doesn't pick between.
Worth its own decision, not assumed here.

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
- **Cross-substrate identity binding.** Resolved in shape, not yet
  built — see "Cross-substrate identity: DID stable, endpoint rotates,
  binding is a record" above. Android's auth path is decided (OAuth,
  public-client mode — see that section for the confidential-vs-public
  correction and the App Link redirect it needs). What remains open is
  purely implementation on the Android side (the native `HttpClient`/
  store shims, the public-client `AtprotoClientMetadata`, the App Link),
  the binding fact's own lexicon/record shape, and CLI's own,
  still-undecided auth mechanism (app-password, a loopback-server OAuth
  flow, or something else — see that section) — separate from the
  binding's own shape.
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
