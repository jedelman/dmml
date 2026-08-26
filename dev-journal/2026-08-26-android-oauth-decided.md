# Android's auth path: OAuth, and a real correction along the way (2026-08-26)

Continuing straight on from the identity-binding pass: Jason decided
Android's PDS auth path is OAuth, not the app-password alternative
named as an open option. Right call for a distributed consumer app —
revocable, no long-lived shared secret typed into or shipped with the
app.

Checked the reuse story the same way the DID-resolver one was checked
earlier today, rather than assuming it: `written-world/server/src/
atproto/oauth_wire.rs`'s `WwOAuthClient = atrium_oauth::OAuthClient<
WwStateStore, WwSessionStore, WwDidResolver, WwHandleResolver,
WorkersHttpClient>` is generic over all five of those type parameters —
the actual OAuth protocol machinery (PAR, PKCE, DPoP, token exchange)
is real, audited `atrium-oauth` code already proven live, not something
Android needs to reimplement. Same shape of win as the DID resolver:
Android needs its own `HttpClient` (shared with that resolver shim) and
its own `StateStore`/`SessionStore` backed by Android's own secure
storage, not new protocol logic.

**One real thing this check caught that a "just reuse it" assumption
would have missed**: this deploy's `atproto_client_metadata` configures
a *confidential* client — `token_endpoint_auth_method: PrivateKeyJwt`,
authenticated with a server-held private signing key (`app_jwk`,
`APP_KEY_ID`). That's fine for a Worker, which genuinely holds the key
privately. It's not fine for Android: a "private" key embedded in a
distributed, decompilable APK isn't private, and shipping the Worker's
exact client config into the app would defeat the entire point of
`private_key_jwt` authentication. Android needs atproto's *public*-
client path instead (PKCE only, no client assertion) — same
`atrium_oauth::OAuthClient` machinery, a different `AtprotoClientMetadata`/
`AuthMethod`. Whether `atrium-oauth` 0.1.7 exposes that mode directly
is flagged as unverified, not assumed, in `ARCHITECTURE.md` — a real
first check for whoever scopes the Android work, not something to take
on faith because the Worker's version compiles.

Also named, not yet designed: Android's OAuth redirect can't reuse
`{public_url}/oauth/callback` (the Worker's own server-side callback
handler) — native OAuth per RFC 8252 needs a verified HTTPS App Link
that the OS routes back into the app, meaning a `.well-known/
assetlinks.json` for Digital Asset Links verification becomes new,
real infrastructure. The `client_id` itself can still point at the
existing `{public_url}/client-metadata.json` — that's static discovery
metadata, unaffected by which auth method the client actually uses.

**CLI's auth path is explicitly left open, not swept in under
Android's decision.** OAuth (even public-client) assumes a system
browser and a routable redirect, which isn't obviously available in
every CLI context. Named three real options without picking one: an
app-password session, a loopback-HTTP-server OAuth flow (the `gh auth
login` shape — open a local port, launch the system browser, catch the
redirect there), or something else. Worth its own decision later, not
assumed by analogy to Android's.

Updated `ARCHITECTURE.md`'s "Cross-substrate identity" section in
place: Android's OAuth decision, the confidential-vs-public-client
correction, the App Link requirement, and CLI narrowed to its own
still-open question. Also fixed a stale forward-reference ("see ...
below") left over from the previous pass, now that this section sits
above "Open design work" rather than needing to point ahead to it.
