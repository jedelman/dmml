# `DMML.AndroidBridge` verified on a real running Android runtime: `jgitCommit` and `atprotoResolve`, both real

Direct continuation of `2026-09-08-android-ndk-cross-compile-of-androidbridge.md`
(which stopped at real `.so`s, `nm`-confirmed symbols, not yet launch-
verified) and the cloud agent's handoff item 3: on-device verification
of `jgitCommit`, then `atprotoResolve`/`atprotoPull`, then
`llmChatComplete`, in that order, "including if something doesn't
work the way the desktop-JVM proof suggested it would." Two things
didn't, and both are real, disclosed bugs, not silently routed around.

## The skeleton (handoff item 2)

`org.jasonedelman.writtenworld.NativeBridge` (new package -- the exact
one `cbits/android_onload.c`'s `BRIDGE_CLASS` hardcodes) declares all
six `external fun`s from `android_onload.c`'s `bridgeMethods` table,
loading a *separate* `.so` (`dmmlandroidbridge`, both ABIs built the
same day) from `org.writtenworld.androidpoc.DmmlBridge`'s existing
`dmmlbridge` -- the two JNI architectures (pure-Haskell v1 vs. this
upcall-based one) haven't been unified into one linked `.so` yet, so
both ship side by side in `jniLibs/` rather than either being torn out.
A new `VerifyActivity` (two buttons, a scrolling log) exercises real
calls; `AndroidManifest.xml` needed a real, previously-missing
`INTERNET` permission -- without it, every real network call under
`atprotoResolve`/`atprotoPull` would have failed regardless of anything
else here.

## Bug 1, real: JGit 6.10 needs an Android API level this app didn't have

First `jgitCommit` attempt, on the API 28 emulator this session's own
prior cross-compile work already had running:

```
JgitException: Git.open(...): java.lang.NoSuchMethodError:
No virtual method readNBytes(I)[B in class
Lorg/eclipse/jgit/util/io/SilentFileInputStream;
```

The exception-marshaling plumbing itself worked exactly as designed --
a real Haskell exception, caught by `guardedRun`, marshaled as an
`"ERROR: ..."`-prefixed `CString`, round-tripped through the JNI
upcall, displayed cleanly in Kotlin. The underlying call failed for a
real reason: `InputStream.readNBytes(int)` is a JDK 11+ method JGit
6.10 assumes exists; Android/ART's own curated `java.io.InputStream`
doesn't add it until API 33. Exactly the same *class* of problem as
`java.net.http.HttpClient` not existing on Android at all (see
`dmml-hs/src/DMML/Http.hs`'s own module haddock) -- Android's core
library is a curated subset of the JDK, not kept in sync with it, and
this is the second time this exact category of gap has been hit and
proven for real rather than assumed away.

Fixed by installing a real `android-34;google_apis;x86_64` system
image (`sdkmanager`, license auto-accepted the same way this session's
earlier phases already established) and a fresh AVD -- not by patching
JGit or catching the exception, since neither addresses the real
constraint. **Disclosed, not fixed**: a real device at API 28-32 would
hit this same crash; `minSdk = 28` (set for the `.so`'s own
`getentropy()` requirement) is not actually sufficient for the JGit
path specifically. Bumping `minSdk` to 33, or finding/patching a JGit
build that avoids `readNBytes`, is real follow-up work, not done here.

## Bug 2, real: OkHttp was never a real Android Gradle dependency

Second `atprotoResolve` attempt, first one after the JGit fix, took
down the entire app process -- not a caught Haskell exception this
time:

```
JNI DETECTED ERROR IN APPLICATION: JNI NewStringUTF called with
pending exception java.lang.ClassNotFoundException: Didn't find class
"okhttp3.Request$Builder" on path: DexPathList[...]
  at org.jasonedelman.writtenworld.NativeBridge.atprotoResolve(...)
Runtime aborting...
```

The `-fno-code` typecheck, the WSL2 host-JVM link, and the real GET/POST
smoke test earlier the same day (`2026-09-08`'s OkHttp migration work)
all genuinely proved `DMML.Http`'s OkHttp calls work -- against a
classpath the desktop CLI supplies by hand
(`okhttp.jar:okio.jar:kotlin-stdlib.jar`, verified via `javap` against
real downloaded jars). That verification never touched the Android
side's actual dependency mechanism at all -- a real, previously-flagged
open question ("how does OkHttp's jar get bundled into the Android
app's classpath... not yet investigated for this specific new
upcall/AndroidBridge architecture") that stayed open until this exact
crash forced it. Fixed with one real Gradle dependency,
`implementation("com.squareup.okhttp3:okhttp:4.12.0")` in
`android-poc/android/app/build.gradle.kts` -- okio and kotlin-stdlib
come transitively, unlike the desktop CLI's classpath which needs all
three named explicitly.

## Verified, for real, on the API 34 emulator, after both fixes

- `jgitCommit("<app-files>/verify-jgit-repo", "verify.txt", "hello from the android JNI upcall, real commit", "on-device verification commit")`
  -> `OK:c49c6fb823e6a7eca7f9b368eda59fb0361b7119`, a real commit SHA,
  a real file on real on-device storage, through Kotlin -> JNI upcall
  -> `DMML.AndroidBridge` -> `DMML.Jgit` -> real JGit -> a real git
  repository (initialized Kotlin-side via `Git.init()` first --
  `NativeBridge` deliberately has no `jgitInit`, matching
  `DMML.AndroidBridge`'s own stated division of labor: repo creation
  is the caller's out-of-band job).
- `atprotoResolve("bsky.app")` ->
  `{"did":"did:plc:z72i7hdynmk6r22z27h6tvur","pdsEndpoint":"https://puffball.us-east.host.bsky.network"}`
  -- a real, live DID + PDS-endpoint resolution against the real
  network, through the same upcall chain, now actually exercising
  OkHttp's real classes (confirmed by the crash *before* the fix, and
  the real JSON *after* it).

Screenshots of both, on-device, are the actual proof -- not narrated
here from a log line.

## What's still open, honestly

- `atprotoPull`, `atprotoCreateSession`/`atprotoCreateRecord`, and
  `llmChatComplete` -- items 2-4 of the handoff's real-verification
  order -- are NOT exercised. `atprotoPull` needs a real, existing
  atproto record collection this session doesn't have a target for;
  `atprotoCreateSession`/`Record` need real atproto credentials;
  `llmChatComplete` needs a real BYOK OpenRouter API key. None
  fabricated, none silently skipped -- `NativeBridge` declares all six
  (RegisterNatives requires the exact full table or none of it binds),
  `VerifyActivity` only wires up buttons for the two that need no
  external credentials.
- `minSdk = 28` is real for the `.so`'s own requirement but NOT
  sufficient for JGit's `readNBytes` call on a real device --
  disclosed above, not fixed. A real device shipped at API 28-32 would
  crash exactly like this session's first emulator attempt did.
- The two upcall/pure-Haskell JNI architectures (`dmmlbridge` /
  `dmmlandroidbridge`) remain two separate linked `.so` files, not
  unified -- carried over from the previous entry's "what's still
  open," still true.
- Handoff item 4 (`Broker.hs`'s `incorporate` orchestration, ported to
  a parallel Android-callable form) is not started.
