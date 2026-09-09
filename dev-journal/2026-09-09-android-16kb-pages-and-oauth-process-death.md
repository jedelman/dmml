# 16KB page-size alignment, and OAuth surviving a real process death

Jason paired his real phone (a Pixel 8) to test the OAuth login screen
for real. Two real, on-device findings, neither hypothetical.

## 16KB page-size alignment

Pixel 8 has a switchable 16KB-page developer option (Google's own
early-adopter testing mechanism ahead of it becoming mandatory).
Checked our own cross-compiled `.so`s against the NDK's own
`libc++_shared.so` via `readelf -l`: the NDK's own library's LOAD
segments are already `0x4000` (16KB)-aligned; ours were `0x1000` (4KB)
-- our GHC-driven `-shared` link never requested the alignment.

Fixed by adding `-optl-Wl,-z,max-page-size=16384` to both the real
`libdmmlandroidbridge.so` link and the stub `libjvm.so` build,
re-verified via `readelf -l` (all LOAD segments now `0x4000`). The
*other*, separate 16KB requirement -- APK zip-entry alignment, not ELF
segment alignment -- was already satisfied automatically by this AGP
version's packaging (`zipalign -c -P 16` passed clean on every native
lib in the APK before this fix, confirming that requirement was never
the gap).

Real, on-device confirmation came from Android's own debug-build
"Android App Compatibility" dialog, which lists per-library ELF-
alignment status by name. Notable: it separates two real error
classes -- `libdmmlbridge.so` (the untouched, genuinely-unaligned old
library) showed the specific "LOAD segment not aligned"; the libraries
this fix touched showed "Unknown error" instead, the same message the
NDK's own always-correctly-aligned `libc++_shared.so` also showed --
strong evidence "Unknown error" here is an unrelated checker quirk (API
37 is very new), not a real alignment failure, though not confirmed
with full certainty.

Not yet confirmed: the phone's `getconf PAGE_SIZE` reported 4096 at
test time -- the 16KB developer option may not have actually been
toggled/rebooted into. The fix is safe regardless (16KB-aligned
libraries load fine on 4KB-page devices too), so applied it anyway
rather than wait to confirm.

## A real process death mid-login, and what it revealed

First real end-to-end login attempt on the phone: real ~76-second
interaction with Bluesky's actual login page, then "logged in and it
crashes immediately." Tracing actual process IDs and timestamps in
`logcat` (not guessing) found the real sequence:

1. `ActivityTaskManager: Force removing ActivityRecord{... VerifyActivity ...}: app died, no saved state` -- Android killed the app's backgrounded process while the Custom Tab had focus. Real, normal OS behavior under memory pressure, not a bug -- and exactly the limitation `OAuthCallbackActivity`'s own doc comment already named as a known gap, now confirmed to actually happen in practice, not just in theory.
2. The redirect still arrived (Chrome redelivered it a few times for the one login attempt -- same `request_uri` in each `ActivityTaskManager` log line, not three separate attempts). A fresh, cold-started process received it with no `OAuthPendingAuthHolder` state -- correctly toasted "no pending login," not a crash.
3. With the original task gone, Android fell back to relaunching the app via its **launcher** activity -- which was `MainActivity`, the old, separate GameScreen demo. That crashed immediately (`UnsatisfiedLinkError: stg_SRT_1_info` from `libdmmlbridge.so`, a real, pre-existing, unrelated bug -- that library was never rebuilt with the RTS whole-archive linking fix `libdmmlandroidbridge.so` already has), and Android's own crash-recovery loop kept relaunching and re-crashing it.

Two real fixes, not a workaround for either:

- **`AndroidManifest.xml`**: moved the `LAUNCHER` intent-filter from
  `MainActivity` to `VerifyActivity`. `MainActivity`'s own bug is real
  and still open (disclosed in a manifest comment), but there's no
  reason for Android's launcher-fallback path to land on a screen
  nobody's testing today. `MainActivity` itself is untouched, still
  reachable, just no longer the default entry point.
- **`OAuthPendingAuthStore.kt`** (new): the persisted counterpart to
  `OAuthPendingAuthHolder`'s in-memory-only design.
  `AtprotoOAuthClient.beginLogin` now saves the `PendingAuth` here
  (`EncryptedSharedPreferences`, same pattern as `ApiKeyStore` --
  carries the PKCE `code_verifier`) *before* opening the Custom Tab,
  not after. `OAuthCallbackActivity` tries the fast, same-process,
  in-memory path first; if that's empty (the real case above), it
  falls back to the persisted entry -- matched by `state`, refusing a
  mismatched or stale one -- completes the real token exchange itself,
  saves the session via `OAuthTokenStore` directly, and shows a Toast.
  There's no live `LoginScreen` coroutine left to notify in this path
  (its whole process died), so the user sees "Already logged in as
  ..." the next time `LoginScreen` composes (it reads
  `OAuthTokenStore` on init) rather than a live status update -- a
  real, disclosed UX gap, not a silent one.

## What's still open

- Whether the phone was actually running with 16KB pages active at
  test time is unconfirmed (`getconf PAGE_SIZE` said 4096).
- The "Unknown error" vs. "LOAD segment not aligned" distinction in
  Android's compatibility dialog is inferred, not confirmed against
  Android's own source for API 37's specific checker implementation.
- `MainActivity`/`libdmmlbridge.so`'s real bug is still open -- just no
  longer the default launch target.
- The actual login has not yet been confirmed to complete successfully
  end-to-end with these fixes in place (built and installed on-device;
  the phone's wireless ADB connection dropped before a fresh attempt
  could be observed).
