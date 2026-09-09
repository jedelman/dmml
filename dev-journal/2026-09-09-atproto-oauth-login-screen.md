# A real atproto OAuth (PAR + PKCE + DPoP) login screen for Android

Jason asked for an atproto login screen and specifically chose full
OAuth over the already-working app-password path
(`NativeBridge.atprotoCreateSession`) when offered the choice --
OAuth is real, substantial new scope (Bluesky's own app is React
Native; no native Android/Kotlin reference implementation exists
anywhere, confirmed by listing every `bluesky-social` GitHub repo and
checking a search result's claimed Android sample directly, which
turned out not to exist -- the search summary was simply wrong).
Built against the real spec (https://atproto.com/specs/oauth,
https://docs.bsky.app/docs/advanced-guides/oauth-client), verified by
fetching both before writing any code, not assumed from generic OAuth
knowledge -- atproto has real, specific requirements generic OAuth
doesn't: PAR is mandatory, DPoP is mandatory for every client type on
every request (not just token exchange), and a native client's
redirect_uri must be a custom scheme in reverse-domain order matching
client_id's own host.

## The one real external dependency: a hosted client-metadata.json

atproto has no native-app exception here -- `client_id` must be a
real, publicly-fetchable HTTPS URL serving the client's metadata JSON,
200 status, `application/json`. Asked Jason which domain; he chose
`jason-edelman.org` (already carries this identity's real
`.well-known/atproto-did`). Added `oauth/written-world-android/
client-metadata.json` there (public client: `token_endpoint_auth_method:
none`, `application_type: native`, `dpop_bound_access_tokens: true`)
and wired it into `scripts/build.mjs`'s static-copy list -- committed
to a new branch (`oauth-client-metadata-written-world-android`), NOT
pushed or merged, since that repo auto-deploys to a live public domain
on push and that's a real decision for Jason to make, not mine.
Redirect URI: `org.jason-edelman.written-world:/callback`, the reverse-
domain scheme the spec requires, registered as an `intent-filter` on
a new `OAuthCallbackActivity`.

## The Android implementation

- `DpopKeyManager.kt`: generates an EC P-256 keypair in
  `AndroidKeyStore` (private key never leaves it, never exported as
  bytes), builds the public JWK, and signs DPoP proof JWTs (RFC 9449).
  Two real, on-device-confirmed gotchas along the way (see below).
- `AtprotoOAuthClient.kt`: resolves the handle/DID to a DID + PDS via
  the EXISTING, already-verified `NativeBridge.atprotoResolve` upcall
  (reused, not reimplemented) -- then PDS -> `.well-known/oauth-
  protected-resource` -> authorization server -> `.well-known/oauth-
  authorization-server` -> PKCE (S256) -> Pushed Authorization Request
  -> Custom Tabs authorization URL. Handles the real DPoP-nonce retry
  atproto's authorization servers enforce in practice (first attempt
  returns `400 use_dpop_nonce` + a `DPoP-Nonce` header; retry once with
  it) at both the PAR and token endpoints independently, since each
  enforces its own nonce.
- `OAuthCallbackActivity.kt` + `OAuthPendingAuthHolder.kt`: catches the
  custom-scheme redirect, exchanges `code` for tokens off the main
  thread (a real `NetworkOnMainThreadException` risk, not
  hypothetical). `PendingAuth` (carries the PKCE `code_verifier`) is
  handed across via an in-memory singleton, not an Intent extra --
  deliberately, so the verifier never appears in a system-visible
  Intent log. Real, disclosed limitation: if the OS kills this
  process while the Custom Tab has focus, the in-memory state is lost
  and login must restart -- a production app would persist
  `code_verifier`/`state` instead.
- `OAuthTokenStore.kt`: access/refresh tokens + DID + PDS + AS issuer
  in `EncryptedSharedPreferences`, same pattern as `ApiKeyStore`. The
  DPoP private key itself is never here -- it stays in Keystore.
- `LoginScreen.kt`: handle input, Log in / Log out buttons, wired into
  `VerifyActivity`.

## Two real bugs found and fixed via actual on-device runs

1. `Signature.initSign(privateKeyEntry().privateKey as ECPrivateKey)`
   threw `ClassCastException: AndroidKeyStoreECPrivateKey cannot be
   cast to java.security.interfaces.ECPrivateKey` -- Keystore's private-
   key handle is opaque and only implements `java.security.PrivateKey`
   (which is all `Signature.initSign` actually needs), not the
   EC-specific interface. Fixed by dropping the unnecessary cast.
2. A DER-to-raw-r||s ECDSA signature converter was needed and written
   from scratch (`Signature.sign()`'s ASN.1 DER output isn't what JWS
   ES256 requires) -- confirmed working by getting past it to the next
   real failure, not by inspection alone (a wrong DER offset/length
   read would show up as a nonce-retry-then-signature-verification
   failure from the server, not the `invalid_client_metadata` seen).

## Verified, for real, on-device, as far as it can go without the metadata deployed

```
Login failed: java.lang.IllegalStateException: PAR failed after nonce retry: 400
{"error":"invalid_client_metadata","error_description":"Unable to obtain
client metadata for \"https://jason-edelman.org/oauth/written-world-android/
client-metadata.json\": Not Found"}
```

This is the *correct*, expected failure -- not a bug. It proves,
against a real authorization server (bsky.social's), that DID/PDS
resolution, AS metadata discovery, PKCE generation, DPoP keypair
generation, ES256 signing (DER->raw conversion included), JWK
construction, and the nonce-retry handling at PAR all work correctly
end to end. The only thing between this and a real login is deploying
the branch above.

## Deployed, and the real flow verified all the way to a human's own password

Jason approved deploying: merged the branch to `jason-edelman.org`
`main` and pushed -- Cloudflare's own git integration auto-deploys on
push (no GitHub Actions workflow file, which is why none was found by
searching for one; confirmed by watching `client-metadata.json` go
from 404 to a real 200 within about a minute of the push).

First real attempt after deploying still failed at PAR, but with a
new, more precise, entirely real error from bsky.social's own
authorization server:

```
{"error":"invalid_redirect_uri","error_description":"Private-Use URI
Scheme redirect URI, for discoverable client metadata, must be the
fully qualified domain name (FQDN) of the client_id, in reverse order
(org.jason-edelman:)"}
```

The original `redirect_uris` (`org.jason-edelman.written-world:/callback`)
had an extra path segment the spec's prose alone hadn't made obvious
was disallowed -- the real server was more precise than the docs.
Fixed in three places that all have to agree exactly: `client-metadata.json`
(redeployed), `AtprotoOAuthClient.REDIRECT_URI`, and the manifest's
`intent-filter` scheme -- all now `org.jason-edelman:/callback`.

After that fix, redeployed and re-ran the flow: **PAR succeeded for
real**, the Custom Tab launched (through Chrome's real first-run setup
on this fresh emulator -- `uiautomator dump` was needed to find the
real "Use without an account" button coordinates after two guessed-
coordinate taps missed), and it landed on `bsky.social`'s own real,
live "Sign in" page -- HTTPS lock icon, real identifier field
pre-filled with the handle tried (`bsky.app`), real password field,
real "Verify the website address" warning. This is the actual, correct
terminus of automated verification: a human's real password belongs
nowhere in this loop, so the flow was cancelled there rather than
proceeding further. Every step up to that point -- resolution, AS
discovery, PKCE, DPoP signing, PAR, the Custom Tab handoff -- is now
confirmed working against the real, live authorization server, not
assumed.

To complete a REAL login (not yet done): use a handle with real,
enterable credentials -- Jason's own (the `did:plc:zz4wcje4a2nbbtc7pdoth3f2`
this domain's `.well-known/atproto-did` already names), typed by Jason
himself into that real bsky.social page, never into this app.

## A real login, completed for real (2026-09-09, later the same day)

Two more real, on-device bugs surfaced testing on Jason's own Pixel 8,
both fixed and both real, not hypothetical -- see
`2026-09-09-android-16kb-pages-and-oauth-process-death.md` for the
16KB-page-alignment and launcher-activity fixes, and the `apply()` ->
`commit()` persistence fix below.

**`apply()` vs `commit()`, a real race**: `OAuthPendingAuthStore.save`
used `SharedPreferences.Editor.apply()`, which schedules its write
asynchronously and returns immediately.
`AtprotoOAuthClient.beginLogin` calls `save()` right before opening
the Custom Tab, which backgrounds this app's process almost
immediately after -- a real race between the async disk write and
Android considering the process killable, reproduced on Jason's phone
as "OAuth redirect arrived with no matching pending login" even
though `save()` had definitely run. Fixed by switching both
`OAuthPendingAuthStore.save` and `OAuthTokenStore.save` to `commit()`
(blocking) -- both already run on `Dispatchers.IO`, so blocking is
safe, and both are exactly the writes that need to survive a
near-immediate process kill.

**A real, correct login completed end to end** on the emulator with
`claude.jason-edelman.org` (its own real DID,
`did:plc:5y6kop75jnvkbujbubrhj6e3`, resolved via `.well-known/atproto-did`
the same way `jason-edelman.org`'s own does) -- confirmed via added
diagnostic logging (tag `AtprotoOAuth`), not just a UI glance:

```
beginLogin: generated state, persisted, opened Custom Tab
[~71 real seconds later]
OAuthCallbackActivity.onCreate: real redirect with real code
OAuthPendingAuthHolder.take() -> HIT (fast path, in-memory, process stayed alive)
completeLogin: Success -- real DPoP-bound access token (typ "at+jwt", alg ES256,
  cnf.jkt = this device's real DPoP key thumbprint), real refresh token,
  pdsEndpoint=https://discina.us-west.host.bsky.network, authServerIssuer=https://bsky.social
saved session, cleared pending store
```

**A real, separate wrinkle, not a bug**: after that real success,
Bluesky's own "Login complete... You are being redirected..."
interstitial page didn't auto-navigate (a real, plausible Chrome
anti-abuse policy: automatic, non-user-gesture navigation to a custom
URI scheme can be blocked, requiring an actual tap -- Bluesky's own
page provides a "Click here if nothing happens" fallback link for
exactly this). That link, tapped three more times over the next ~3
minutes, redelivered the SAME already-consumed code/state each time
(`OAuthCallbackActivity.onCreate` logged three more times with
identical `code=cod-9567...`/`state=wS35...` params) -- each correctly
refused (`OAuthPendingAuthHolder.take() -> MISS`,
`OAuthPendingAuthStore.loadMatching -> MISS`, since the entry was
already cleared by the real first success) rather than replaying a
used authorization code. Real, correct security behavior, not a
retry-worthy failure -- momentarily confusing on screen ("pending
redirect error again!") since nothing on this build's UI yet
distinguishes "replayed, already-used code, you're actually fine" from
a genuine failure. Jason confirmed the session was real and live:
cancelled the stale browser tab, reopened the app, saw "Already logged
in as did:plc:5y6kop75jnvkbujbubrhj6e3" (LoginScreen reading
`OAuthTokenStore` on init, exactly as designed).

## What's still open
- **DPoP on every subsequent API call**: the spec's real requirement
  --"applies to every PDS request", not just login -- means
  `DMML.Atproto`'s `createRecord`/`pullNewRecords`/etc. (currently
  plain `Authorization: Bearer`) would need DPoP-proof headers added
  per request, using the access token's `ath` claim, threaded through
  `DMML.Http`. Real, substantial, cross-layer follow-up work,
  explicitly not done here -- this entry covers login only.
- **Token refresh**: `refresh_token` is stored but nothing yet uses it
  to refresh an expired `access_token`.
- **`OAuthCallbackActivity`'s in-memory `PendingAuth` limitation** is
  now mitigated (persisted fallback via `OAuthPendingAuthStore`, see
  above), not eliminated -- a process kill during the disk-fallback
  path itself, or before `commit()` returns, is still a real, if much
  smaller, gap.
- **The stale-redirect-replay UX**: a real, correctly-refused replay
  (see above) currently surfaces as the same generic toast as an
  actual failure. Worth distinguishing on screen -- "already logged in"
  vs. "login failed" -- as real follow-up polish, not correctness work.
- Confirmed working end to end on the emulator only; the phone
  (Jason's real Pixel 8) has the same build installed but a full,
  real login there hasn't been separately confirmed yet.
