# A better native crate than the reuse-atrium plan (2026-08-26)

Jason pointed at `atproto-oauth` (crates.io) as "the OAuth module we
need." Checked it for real rather than taking the name at face value —
crates.io's own page is a JS SPA that WebFetch can't render, so pulled
the crate metadata and README straight from crates.io's API (`curl`,
not a browser). Real, active, well-scoped: Nick Gerakines, MIT, 27
published versions, most recent April 2026, part of a two-crate family
with `atproto-identity` (DID/handle resolution, key operations) — same
author, same repo, released together.

**This corrects the recommendation from earlier in the same design
pass, not just confirms it.** The plan had been: reuse `atrium-identity`
natively by writing a new `HttpClient` transport shim for CLI/Android,
since `CommonDidResolver<T: HttpClient>` is already generic over its
transport. That's true and would have worked — but checking
`atproto-identity`'s own README showed something better: its
`resolve_subject` takes a bare `reqwest::Client` argument, not a
generic transport trait. It's written assuming a native runtime from
the start, not abstracted to also fit inside a wasm32 Worker sandbox
the way `atrium-identity` had to be. CLI/Android don't need to write a
shim for a generic that exists to solve a constraint they don't have —
they can just depend on a crate that already assumes their actual
runtime.

Same logic, sharper payoff, on the OAuth side. The Worker's
`WwOAuthClient` (`atrium_oauth::OAuthClient<...>`) is real and proven
live, but it's built around this deploy's own *confidential*-client
config (`PrivateKeyJwt`, a server-held signing key) — flagged last turn
as needing correction for Android regardless of which crate got used,
since that specific config can never ship in an APK. `atproto-oauth`'s
README shows it doesn't bundle one opinionated `OAuthClient` struct at
all — PKCE (`pkce::generate()`), DPoP (`dpop::auth_dpop`, with
automatic nonce-retry handling, the same fiddly RFC 9449 requirement
`atrium-oauth`'s own `DpopClient` also takes seriously), OAuth discovery
(RFC 8414), and JWT mint/verify are separate, composable primitives.
Assembling a public-client flow (PKCE, no JWT client assertion) is just
using a subset of those primitives, not fighting a client struct's own
design center the way retrofitting `atrium-oauth`'s `OAuthClient` might
have required — resolves last turn's "unverified whether atrium-oauth
0.1.7 exposes public-client mode" flag by routing around the question
rather than answering it.

Named plainly, not glossed over: this leaves two independent AT
Protocol client implementations in the system (`atrium-*` in the
Worker, `atproto-identity`/`atproto-oauth` in CLI/Android) — a real
cost (two dependencies to track, two possible sources of spec-drift),
worth stating rather than pretending "just reuse atrium everywhere"
was free. It wasn't; the Worker's own reasons for needing
transport-generic, wasm32-safe crates don't apply to CLI/Android, and
forcing one dependency choice across both would have meant writing new
shim code to route around a constraint neither of them actually has.

Updated `ARCHITECTURE.md`'s "Cross-substrate identity" section in
place: replaced the atrium-reuse recommendation for both DID resolution
and OAuth with the `atproto-identity`/`atproto-oauth` plan, kept the
Worker's own `atrium-*` usage completely untouched (no reason to touch
what's already live and proven), and pointed CLI's still-open auth
decision at the same crate family if it ends up choosing OAuth too —
one native crate family for both native clients, not a second bespoke
one per client.
