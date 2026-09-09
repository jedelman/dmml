# DPoP threaded through to a real authenticated write

Follow-up to the real OAuth login success earlier the same day
(`2026-09-09-atproto-oauth-login-screen.md`): the disclosed gap was
that `DMML.Atproto`'s write path (`createRecord`) only ever sent plain
`Authorization: Bearer`, which a DPoP-bound OAuth access token can't
use -- the atproto spec requires `Authorization: DPoP <token>` plus a
per-request `DPoP: <proof-jwt>` header, proof-signed fresh for every
single call.

## Design: sign in Kotlin, orchestrate in Haskell

The DPoP private key lives in Android's own Keystore, generated
non-exportable on purpose (`DpopKeyManager.kt`, built for the OAuth
login flow) -- there is no raw key material to hand to Haskell even if
that were desirable. Two ways to make Haskell's `DMML.Http` produce
DPoP-signed requests: reimplement Keystore access + ECDSA signing +
JWK/JWT construction as raw JNI primitive calls in Haskell (a second,
more fragile copy of already-proven Kotlin logic), or have Haskell
call back into the EXISTING `DpopKeyManager.createProof` via the same
generic JNI upcall machinery `DMML.Jgit`/`DMML.Http` already use for
JGit/OkHttp objects. Chose the second -- no new crypto code anywhere,
Haskell just asks Kotlin for a proof, the same way it already asks
OkHttp to make a request.

One real friction point: Kotlin default parameter values
(`createProof(htm, htu, nonce: String? = null, accessToken: String? =
null)`) compile to a synthetic `$default` bridge method taking an
extra bitmask + marker `Object` arg -- awkward to construct correctly
from raw `GetStaticMethodID`/`CallStaticObjectMethod` calls for no
real benefit. Added a separate `@JvmStatic createProofForNative(htm,
htu, nonce, accessToken)` overload instead, using empty-string
sentinels for "absent" rather than passing a real Java `null` jstring
across the boundary.

## What got built

- `DMML.Jni`: one new generic primitive,
  `c_callStaticObjectMethod4Obj` (4 object/string args) -- everything
  else DPoP needed was already covered by existing primitives
  (`Response.header(String)` for reading the `DPoP-Nonce` response
  header reuses the same shape `c_callObjectMethod1Str` already had).
- `DpopKeyManager.kt`: the one new `createProofForNative` overload
  above, calling the same, already-proven `createProof`.
- `DMML.Dpop` (new module): `createProofUpcall`, the upcall wrapper --
  `FindClass`+`GetStaticMethodID`+the new 4-arg primitive against
  `org.jasonedelman.writtenworld.oauth.DpopKeyManager`. Android-only,
  disclosed as such: this class doesn't exist under an `EmbeddedJvm`
  (desktop CLI), so calling this there fails through the same
  `describeAndClearException` path everything else already uses, not
  silently.
- `DMML.Http.postJsonDpop` \/ `oneRequestDpop` (new): builds the
  request, gets a proof, sends, and on a `400`\/`401` carrying
  `use_dpop_nonce` + a real `DPoP-Nonce` response header, retries
  ONCE with a freshly re-signed proof including that nonce -- the
  identical real-server behavior `AtprotoOAuthClient.kt`'s own
  PAR\/token-endpoint calls already discovered and handled on
  2026-09-09, now needed again here because the PDS (resource server)
  enforces its own nonce independently of the authorization server's.
- `DMML.Atproto.createRecordDpop` (new): same
  `com.atproto.repo.createRecord` call as `createRecord`, but takes a
  raw `(pdsEndpoint, did, accessToken)` triple from a completed OAuth
  session instead of an app-password `Session` record.
- `DMML.AndroidBridge.android_atproto_create_record_dpop` +
  `atprotoCreateRecordDpopBridge`, `cbits/android_onload.c`'s matching
  `RegisterNatives` entry, `NativeBridge.atprotoCreateRecordDpop` --
  same shape as every other function in this file.
- `VerifyActivity`: a 5th check button, reading the saved
  `OAuthTokenStore` session and publishing a real, clearly-labeled
  test record (`test/dpop-verification` predicate) if logged in;
  a clean `SKIPPED` message, not a crash, if not.

Only `createRecord`\/`deleteRecord` ever carried auth at all --
confirmed by re-reading `DMML.Atproto.hs` before starting, not assumed:
`resolveHandle`, `listRecords`\/`pullNewRecords` all hit public,
unauthenticated XRPC endpoints already. `deleteRecordDpop` isn't built
yet (real, disclosed, same-shape follow-up).

Both `.so`s (arm64-v8a, x86_64) recompiled and relinked with the
already-solved toolchain, `-Wl,-z,max-page-size=16384` kept for the
16KB alignment fix from earlier the same day -- no new toolchain
blockers this time.

## Verified, for real, on the emulator, first real attempt

```
atprotoCreateRecordDpop ->
at://did:plc:5y6kop75jnvkbujbubrhj6e3/org.jason-edelman.writtenworld.commit/...
OK, real DPoP-authenticated record published
```

The saved OAuth session (from the login earlier the same day) was
still valid -- a real, live record was published to
`did:plc:5y6kop75jnvkbujbubrhj6e3`'s own repo, through the full chain:
Kotlin Keystore ECDSA signing -> Haskell JNI upcall
(`DMML.Dpop.createProofUpcall`) -> OkHttp POST with real
`Authorization: DPoP <token>` + `DPoP: <proof>` headers -> a real 2xx
from the PDS. First real attempt, no nonce-retry needed this time (the
PDS didn't require one for this particular request), no errors.

## What's still open

- `createRecordDpop`'s test record, published for real above, needs a
  real `deleteRecordDpop` (or manual cleanup via the existing
  app-password `deleteRecord`, which still works against any account
  with app-password auth set up) to remove it afterward.
- `pullNewRecords`\/`listRecords` don't need DPoP (public endpoints),
  confirmed -- not a gap, a real finding.
