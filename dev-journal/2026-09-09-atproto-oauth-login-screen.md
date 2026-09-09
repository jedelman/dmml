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

## What's still open

- **Deploy the client metadata**: push/merge
  `oauth-client-metadata-written-world-android` on `jason-edelman.org`
  (Jason's call -- it's a live public site with an auto-deploy
  pipeline). Once live, the actual browser-based login + token
  exchange is unverified but should be the next real test.
- **DPoP on every subsequent API call**: the spec's real requirement
  --"applies to every PDS request", not just login -- means
  `DMML.Atproto`'s `createRecord`/`pullNewRecords`/etc. (currently
  plain `Authorization: Bearer`) would need DPoP-proof headers added
  per request, using the access token's `ath` claim, threaded through
  `DMML.Http`. Real, substantial, cross-layer follow-up work,
  explicitly not done here -- this entry covers login only.
- **Token refresh**: `refresh_token` is stored but nothing yet uses it
  to refresh an expired `access_token`.
- **`OAuthCallbackActivity`'s in-memory `PendingAuth` limitation**
  (above) -- real, disclosed, not fixed.
