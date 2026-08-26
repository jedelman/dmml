# Cross-substrate identity binding: DID stable, endpoint rotates (2026-08-26)

Continuing the live-deployment design thread from earlier today. Jason's
framing on the last open item — "the DID is stable but the endpoint can
rotate" — turned out to separate two problems that had been sitting
under one label ("cross-substrate identity") in `ARCHITECTURE.md`
since the crate split. Worth naming the separation explicitly, because
conflating them would have meant solving both at once when only one of
them is actually unsolved.

**Problem A: DID → current PDS endpoint.** Already solved, live, in
production — not something this design pass needed to invent. Checked
directly against `written-world/server/src/atproto/identity.rs`:
`WwDidResolver = CommonDidResolver<WorkersHttpClient>` does real,
un-cached DID-document resolution via `atrium-identity` (not
hand-rolled), and `CommonDidResolver<T>` is generic over its own HTTP
transport — `WorkersHttpClient` is a thin shim, not part of the
resolution logic itself. A native CLI/Android client needs the same
`atrium-identity` dependency plus its own native `HttpClient` impl,
reusing the identical, already-audited resolution code the Worker runs
today. This had been implicitly bundled into "cross-substrate identity"
as if it were open; checking the actual code showed it isn't — it's a
scoped implementation task (one transport shim), not a design question.

**Problem B: DID ↔ iroh author key.** This is the actually-new problem,
and it turns out small once Problem A is separated out: since endpoint
resolution happens fresh at use time, the binding fact never needs to
mention an endpoint. It's an ordinary record, published in the player's
own PDS repo via the same `swapCommit`-gated write path every other
commit already uses — proof of authorship is just "only this DID's
controller can write to this DID's repo," the same guarantee atproto
already provides everywhere, no new trust root. Repo migration already
carries the record along under a PDS migration, for free. The binding
is specifically to an iroh **Author** key (`AuthorId`/`AuthorSecret`),
not the Namespace — the namespace is the shared world identity, the
author is the per-device/per-writer key that `(namespace, author, key)`
entries are actually keyed by.

**Rotation needed no new mechanism either** — a lost device or a
routine key change is just another commit, consuming the old binding
fact and producing a new one. Exactly the same consume-old/produce-new
discipline as everything else in the grammar; noticing this took
checking whether the binding fact itself was ordinary DMML content
(it is) rather than assuming it needed special revocation machinery.

Also checked CLI's actual current state before writing any of this
down, rather than assuming it already had partial atproto/iroh
capability: `cli/src/main.rs` is 60 lines, imports `engine::Game`
directly, zero networking of any kind. This is genuinely greenfield —
useful to know precisely because it means there's no existing pattern
to preserve or migrate away from there, but also no shortcuts.

Left genuinely open, flagged as distinct from the binding design above
rather than silently bundled into it: what CLI/Android actually
authenticate to the PDS with. `written-world`'s existing OAuth flow
(`oauth_wire.rs`) is built for a browser redirect and isn't obviously
right for a CLI; an app-password session
(`com.atproto.server.createSession`) is simpler but a different trust
and revocation model. Not decided here — a real, separate next
decision, not assumed as "just reuse OAuth" or "just use app passwords"
without Jason weighing in.

Updated `ARCHITECTURE.md` with a new "Cross-substrate identity: DID
stable, endpoint rotates, binding is a record" section, and narrowed
the corresponding "Open design work" bullet to what's actually still
open (the transport shim, the record's lexicon shape, and the
CLI/Android auth mechanism) now that the binding's own shape is
resolved.
