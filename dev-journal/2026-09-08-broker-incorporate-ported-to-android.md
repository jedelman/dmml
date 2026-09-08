# `Broker.hs`'s `incorporate` ported to Android, verified on-device; BYOK API-key UX added

Direct continuation of `2026-09-08-android-jni-upcall-verified-on-device.md`
-- handoff item 4: "`written-world`'s `Broker.hs` orchestration ported
to a parallel Android entry point." `Broker.hs` turned out not to be
where every dev-journal entry so far assumed.

## Finding `Broker.hs` -- it was never on `main`

`jedelman/written-world`'s default branch (`main`) is a Rust/Cloudflare-
Workers project (`Cargo.toml`, `cli/`, `engine/`, `server/`, `web/`) --
no Haskell, no `Broker.hs`, nowhere in any local checkout of it either.
Every dev-journal entry that reached back into `written-world` for
`Broker.hs`/`Author.hs`/`BrokerSmokeTest.hs` was describing real code
that genuinely doesn't exist on `main`. Asked Jason rather than guess
or fabricate a fresh implementation under the same name; his answer --
"Check the branches! We're not doing this on main" -- was exactly
right: `git fetch`ing every remote branch and searching full history
(`git log --all --diff-filter=A -- '**/*.hs'`) found the real commits
(`317d179` "Add the real atproto broker", `83ef1ec` "Add
written-world-author") on `origin/claude/written-world-dmml-enrichment-257mkv`,
a real, current, un-merged branch with `cli/app/Broker.hs` and everything
else exactly as described.

## The port

`DMML.AndroidBridge.brokerIncorporateBridge` (new) adapts `Broker.hs`'s
`incorporate` the same way every other function in that module already
adapts its own desktop counterpart onto `UpcallJvm` -- not a shared
import (a different repo, a real language/build boundary), a real
re-implementation kept behavior-equivalent on purpose, including two
of the original's own real quirks carried over deliberately rather
than "fixed": `jgitResolve ... "HEAD:commits"` is hardcoded regardless
of the actual `commitsDir` argument, and checkpoint-folding only
triggers when `commitsDir` is literally `"commits"`. One necessary,
disclosed difference: the desktop original prints its divergence
report via `putStrLn` and calls `exitFailure` on a validation
rejection -- neither works across an FFI boundary with no attached
console, so this version returns the divergence report as structured
JSON and a rejection as a normal `Left`, marshaled the same
`"ERROR: ..."` way as every other function here.

New foreign export `android_broker_incorporate` (`JNIEnvPtr -> repoDir
-> peerIdentifier -> cursorFile -> commitsDir -> JSON`), a new
`native_brokerIncorporate` wrapper and `bridgeMethods` table entry in
`cbits/android_onload.c`, and `NativeBridge.brokerIncorporate` in
Kotlin. Both `.so`s (arm64-v8a, x86_64) recompiled and relinked with
the existing toolchain from the previous two entries -- no new
blockers this time, every real one was already solved.

## Verified, for real, on-device

```
brokerIncorporate(repo, "bsky.app", ...) ->
{"incorporatedCount":0,"message":"nothing new","nextCursor":null}
OK, real pull + incorporate attempt
```

`bsky.app` has no records in `org.jason-edelman.writtenworld.commit`
(expected -- it's a real public account, not a `written-world` peer),
so this exercises the real empty-batch path rather than a real
incorporate -- still a real network call, a real JSON round-trip
through the new primitive, no crash. A real populated peer to test the
actual incorporate/commit/divergence/checkpoint path against is real
follow-up work, same category of gap as `atprotoPull`'s own
unverified status in the previous entry.

## BYOK API-key UX, for when a real key is available

`ApiKeyStore.kt` (new): `EncryptedSharedPreferences`
(`androidx.security:security-crypto`, Keystore-backed AES) for the
OpenRouter API key -- a real secret, not stored in plaintext. No key
hardcoded or invented anywhere; Jason supplies the actual value
himself. `VerifyActivity` gained a password-masked input + Save button
and a 4th check button (`llmChatComplete`) that reads the stored key;
with none saved it returns a clean `SKIPPED: no API key saved yet`
message rather than crashing or calling with an empty string --
verified for real on-device (screenshot: clean skip, no crash). Once a
real key is saved, tapping the button exercises the real BYOK chat-
completion path end to end -- not yet done, waiting on a real key.

## What's still open

- `llmChatComplete` itself: UX built and the no-key path verified: the
  actual BYOK call is not yet exercised, waiting on a real API key.
- `atprotoCreateSession`/`atprotoCreateRecord`: still no UI, still need
  real atproto credentials.
- `brokerIncorporate`'s real incorporate/commit/divergence/checkpoint
  path (as opposed to its empty-batch path) is unverified -- needs a
  real peer with real records in the collection.
- The `claude/written-world-dmml-enrichment-257mkv` branch this port
  came from is still unmerged into `written-world`'s `main` -- a real,
  disclosed state of that separate repo, not something resolved here.
