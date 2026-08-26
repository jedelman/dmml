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
  It also owns `substrate::{Substrate, CasSubstrate, AppendSubstrate}`
  — real trait signatures now, built (not just designed) 2026-08-26:
  `Substrate` carries identity/sovereignty-root/reads (including
  `resolve_fact`, the trait-level equivalent of `getResolved`);
  `CasSubstrate` and `AppendSubstrate` each carry exactly the write
  contract its own backend can honor, deliberately not unified into one
  shape (see `substrate.rs`'s own module doc comment for the full
  reasoning). No concrete backend implements either yet — see
  `dmml-substrate-kit` below for the one that does, in-memory only.

- **`dmml-substrate-kit`** — genuinely shared, substrate-*specific*
  tooling that more than one concrete adapter would otherwise
  duplicate: `atproto_cid` (the extracted `CIDv1(dag-cbor, sha2-256)`
  strategy, byte-compatible with a real atproto PDS for predicates, not
  yet fully for subjects/objects — see that module's own doc comment
  for the precise, still-open gap), and now `mock::MockAppendSubstrate`
  — a real, tested, zero-network `AppendSubstrate` implementation
  (`dmml-substrate-kit/src/mock.rs`), proving the trait split is
  actually implementable, not just plausible on paper. Its tests
  exercise the load-bearing claims directly: a bare `produces` never
  retracts anything; two commits consuming the identical base is the
  real, detectable conflict signature (`resolve_fact` reports both);
  writes are genuinely author-partitioned. An `iroh_cid` module
  (wrapping iroh-blobs' raw BLAKE3 as a CIDv1 under the registered
  BLAKE3 multicodec) and a `CasSubstrate` mock (standing in for
  atproto) remain named next steps, not yet built.

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

**The conflict check reuses `getResolved`, not a new query.** Checked
against `dmml-runtime`'s two existing candidates before designing
anything new, and neither is a match on its own: `WorldGraph::
consume_state`/`is_retracted` (`graph.rs`) look like the obvious reuse
target, but their own doc comment rules them out directly — they answer
whole-node currency for a `ConsumeRef::Strong` reference only; a
`FactRef` entry is *deliberately* not fed into that bookkeeping, because
this in-memory graph "has no notion of which specific commit produced
which triple," and says outright that answering *that* question is
`appview`'s job, not this one's. `appview` turns out to already do
exactly that job, live: `org.jason-edelman.writtenworld.getResolved`
(`appview/src/main.rs`) is a real, deployed, non-wasm32 service that
indexes every commit to the collection across every repo via Jetstream
(Bluesky's own, real, deployed relay — not a spike), and its
`Resolver::resolve` walk already computes, per `FactRef`, exactly
whether the cited `(commit, subject, predicate[, object])` is still
current or has been excluded as "retracted or structurally invalid."
This is the general repo-traversal/query primitive the conflict check
needs — not a new one to build, an existing live service to call.

One real caveat, not glossed over: `getResolved`'s index is a
Jetstream-driven read model, current as of whatever it's last consumed,
not a synchronous check against the PDS at the instant of write the way
`swapCommit`'s CAS is. There's a real staleness/TOCTOU window between
resolving "is this key still current" and the checkpoint commit
actually landing. Whether that window is acceptable here (a checkpoint
that turns out to have raced past a retraction just becomes one more
`disputes`-flagged case, not silent data loss, since nothing about this
design depends on the conflict check being infallible) or needs a
narrower re-check immediately before the write is real, scoped, and
much smaller than the "design a whole new query" question this
replaces.

**Resolved: the staleness window is a non-issue outside gaming, and
gaming has its own fix that doesn't touch this design.** Most DMML
applications (this project's own paper-authoring and critique work
among them) have no tight interactive loop where a few seconds of
`getResolved` lag could matter — a checkpoint that resolves a moment
late just disputes a moment late, which was already the accepted
outcome above regardless of cause. The one class of application where
staleness could actually bite is real-time gaming (two players racing
to the same item, a race the player experiences directly and
immediately) — and that doesn't need a stronger *infrastructure*
guarantee either. It's a content-level fix, the same move Section 3 of
`papers/desiring-production-ontology/DRAFT.md` already makes for
lack/desire: DMML's grammar has no opinion about tight-consistency
needs any more than it has one about lack, so a game author who needs
one authors it as ordinary, self-declared content — a TTL-shaped
predicate on the specific facts that actually need tight bounds (an
item pickup, a turn lock), not a change to the conflict check, the
`Substrate` trait, or `getResolved` itself. Nothing here is designed
yet — this is a closed non-issue for the general case and a named,
content-level direction for the one case that isn't, not new
infrastructure work either way.

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

**Endpoint resolution is already real and live on the Worker side, via
`atrium-identity`.** written-world's `server/src/atproto/identity.rs`
doesn't cache a PDS host anywhere — `WwDidResolver =
CommonDidResolver<WorkersHttpClient>` resolves a DID's current
`serviceEndpoint` fresh, confirmed generic over its own HTTP transport
(`CommonDidResolver<T: HttpClient>`), with `WorkersHttpClient` as the
only Workers-specific part.

**CLI/Android don't need a transport shim for that generic, though —
they need a different, already-native crate family.** `atrium-identity`
is built to be transport-generic specifically *because* it has to run
inside the Workers wasm32 sandbox; CLI and Android have no such
constraint and are better served by
[`atproto-identity`](https://crates.io/crates/atproto-identity) (Nick
Gerakines, MIT, actively maintained — 27 published versions as of
`atproto-oauth`, most recent April 2026) instead. Confirmed directly
from its own README: `resolve_subject` takes a bare `reqwest::Client`
as an argument, not a generic transport trait — it's written assuming a
native runtime, not abstracted to also fit inside a wasm32 Worker the
way `atrium-identity` had to be. That's exactly the shape CLI/Android
want: no shim to write at all, just depend on a crate already built for
where they actually run. This supersedes the "write a native
`HttpClient` shim for `atrium-identity`" recommendation from earlier in
this same design pass — checking the real crate changed the answer, not
just confirmed it.

This does mean two independent AT Protocol client implementations exist
across the whole system after this (`atrium-*` in the Worker,
`atproto-identity`/`atproto-oauth` in CLI/Android) — a real, named
cost (two dependencies to track, two possible sources of spec-drift)
traded for each one fitting its actual runtime without fighting
platform constraints or hand-writing a shim. Worth stating plainly
rather than treating "just use one library everywhere" as free — it
isn't, here.

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
for a distributed consumer app.

**The Worker's `atrium-oauth`-based flow was the wrong reuse target
here too, for the same reason as `atrium-identity` above.**
`WwOAuthClient = atrium_oauth::OAuthClient<WwStateStore, WwSessionStore,
WwDidResolver, WwHandleResolver, WorkersHttpClient>` is real, audited,
and proven live — but it's built around this deploy's own **confidential
client** configuration (`token_endpoint_auth_method: PrivateKeyJwt`,
authenticated with a server-held signing key, `app_jwk`). That
configuration cannot be copied into Android at all — a "private" key
embedded in a distributed, decompilable APK isn't private, and shipping
it would defeat the entire point of `private_key_jwt` client
authentication. Android needs atproto's **public-client** path instead
(PKCE only, no client assertion).

Rather than fight `atrium-oauth`'s `OAuthClient` into a shape it may or
may not directly support (unverified, and not the crate's own apparent
design center), the same native-first
[`atproto-oauth`](https://crates.io/crates/atproto-oauth) crate family
that replaces `atrium-identity` above fits this directly: it exposes
PKCE (`pkce::generate()`, RFC 7636), DPoP (`dpop::auth_dpop`/
`request_dpop`, RFC 9449, with automatic nonce-retry handling — the
same fiddly requirement `atrium-oauth`'s own `DpopClient` handles
internally, confirmed both crates take it seriously rather than
hand-waving it), OAuth discovery (`resources::discover_protected_resource`/
`discover_authorization_server`, RFC 8414), and JWT mint/verify as
separate, composable primitives rather than one opinionated client
struct — a public-client flow is just "use PKCE, skip the JWT-assertion
step," not a mode fighting against the crate's own design the way it
would be retrofitting `atrium-oauth`'s `OAuthClient`. Same author, same
crate family, actively maintained (MIT, 0.14.5 as of this check).

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

**CLI's auth path is decided too — and it's two paths, not one, because
the same CLI binary has two genuinely different callers.** A human
running CLI locally has a system browser available; an agent driving
CLI inside a sandbox or container (this project's own Claude Code
sessions included — this very session has no interactive browser to
pop a redirect through) does not. Those aren't the same auth problem in
different clothes; they need different mechanisms, chosen at runtime
(is a browser actually available here), not at build time:

- **Local, human-interactive CLI → OAuth**, same crate and same
  public-client path as Android: `atproto-identity`/`atproto-oauth`,
  with a loopback-HTTP-server redirect (open a local port, launch the
  system browser, catch the callback there — the same shape `gh auth
  login` and similar CLIs already use) standing in for Android's App
  Link. One native crate family serving both native clients'
  interactive flow, not two bespoke implementations.
- **Sandboxed/container/agent-driven CLI → an app-password session**
  (`com.atproto.server.createSession`) — there's no browser to redirect
  through in that context, so OAuth's authorization-code flow isn't
  degraded there, it's unavailable, regardless of which crate implements
  it. A different trust and revocation model (a long-lived shared
  credential, not a revocable token) accepted specifically because it's
  the only one that actually works headless.

Both paths land CLI in the same place either way: an authenticated
session it uses to write the DID↔iroh binding record and its own
checkpoint commits through the same `swapCommit`-gated path everything
else already uses.

## Open design work (named, not designed here)

- ~~`Substrate`'s real method signatures~~ **Built, 2026-08-26**
  (`dmml-runtime/src/substrate.rs`, `dmml-substrate-kit/src/mock.rs`).
  Split honestly rather than unified: `CasSubstrate` for atproto's real,
  already-proven compare-and-swap; `AppendSubstrate` for iroh's
  author-partitioned writes, which need no CAS at all but push the
  concurrent-base conflict check onto the caller as a required
  pre-write step — proven implementable, not just designed, by
  `MockAppendSubstrate`'s own tests (a bare `produces` never retracts;
  two commits consuming the same base is the real, detectable conflict
  signature; writes are genuinely author-partitioned). What's still
  open: a concrete `CasSubstrate` implementation (real, against a live
  PDS — the mock built here only covers the `AppendSubstrate` shape),
  and wiring an application's actual checkpoint loop (build a commit →
  check `resolve_fact` → append or dispute → eventually check-and-write
  to a `CasSubstrate`) on top of these traits.
- **The conflict check needs no new primitive — see "The conflict check
  reuses `getResolved`, not a new query" below.** What remains open is
  narrower than a query design: whether `getResolved`'s Jetstream-driven
  read-model staleness window is acceptable for this use, or whether the
  checkpointing client needs to re-check immediately before the write
  rather than trusting an earlier resolve.
- **Cross-substrate identity binding.** Resolved in shape, not yet
  built — see "Cross-substrate identity: DID stable, endpoint rotates,
  binding is a record" above. Android's auth path is decided (OAuth,
  public-client mode, via `atproto-identity`/`atproto-oauth` rather than
  retrofitting the Worker's `atrium-oauth`-based confidential client),
  and CLI's is decided too, as two runtime-selected paths sharing the
  same crate for its interactive half (OAuth, loopback redirect, for a
  human at a terminal; app-password for a sandboxed/agent-driven
  invocation with no browser — see that section for why these are
  genuinely different problems, not one decision). What remains open is
  purely implementation: wiring `atproto-oauth`'s PKCE/DPoP/discovery
  primitives into the actual public-client and loopback-redirect flows,
  the App Link, the app-password path's own credential storage, and the
  binding fact's own lexicon/record shape.
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
