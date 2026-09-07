# F1 — Android JNI bridge, now carrying the real interpreter

The last open item from `jedelman/dmml#1`. Jason: "go ahead and tackle
F1. keep it as simple as you can." The original PoC here (now superseded,
see below) was the smallest real thing proving a Haskell function is
callable across a JNI boundary at all, following `hatter`'s documented
shape (GHC NDK cross-compile to a `.so`, a thin Kotlin call site that
`System.loadLibrary`s it, a JNI C bridge that boots the RTS and calls
in) — see `written-world`'s
`dev-journal/2026-09-02-platform-pivot-cli-android-filesystem-canonical.md`
for why this path was picked over `reflex-platform`/`obelisk`.

**Updated 2026-09-06**, per `dmml/dev-journal/2026-09-04-android-jni-vs-ipc.md`'s
recommendation (JNI, in-process, calling dmml-hs's own library functions
directly — not a second wrapper around either CLI executable's argv
interface): the bridge mechanism proven by the original fake `hsGreet`
PoC now carries the REAL interpreter. `haskell/Bridge.hs` is gone;
the bridge is `DMML.JniBridge` (`dmml-hs/src/DMML/JniBridge.hs`), a real
module inside the `dmml-hs` package that calls `DMML.Materialize`/
`DMML.Guard`/`DMML.Fire` directly, with every exported function wrapped
in `Control.Exception.try` per that dev-journal entry's required
mitigation (no process isolation on this path — an uncaught exception
crossing the FFI boundary is undefined behavior, not a clean subprocess
exit code).

## What this proves, and what it still doesn't

**This machine has a real Android SDK + NDK (`aarch64-linux-android24-clang`
confirmed present) and a real host GHC 9.10.3 + cabal 3.16.1.0 — but
still no cross-compiling GHC targeting `aarch64-linux-android`, and no
device/emulator.** Same honest split as the original PoC, one layer
further in:

**Verified for real, on host GHC, in this environment:**
- The entire `dmml-hs` library, including `DMML.JniBridge`, builds
  clean via `cabal build lib:dmml-hs` — not just one isolated file, the
  whole real dependency closure (aeson, megaparsec, containers, etc.).
- **The whole FFI-shaped surface a JNI caller would actually use** — not
  just that it compiles — was exercised end-to-end via real
  `Foreign.C.String` marshaling (`dmml-hs/app/JniBridgeSmokeTest.hs`,
  `cabal run jni-bridge-smoke-test`): materializing a world+machine
  fixture, rendering it, enumerating legal actions
  (`DMML.Guard.availableTransitions`), firing a transition that both
  asserts and retracts a fact (`DMML.Fire.fireTransition`, with real
  content-addressed provenance via `DMML.LocalIdentity.localFileRef` so
  the retract can cite something real instead of refusing), and two
  deliberate error paths (an undeclared transition, malformed DMML
  source) — confirming errors come back as plain `"ERROR: ..."` strings,
  never a raw Haskell exception. See
  `dev-journal/2026-09-06-real-jni-bridge-verified-on-host.md` for the
  full transcript.

**NOT verified — needs a real Android toolchain, on a machine that has
one:**
- Cross-compiling `dmml-hs` (its full dependency closure, not one file
  — a materially bigger lift than the original single-file PoC) to
  `arm64-v8a` via a real `aarch64-linux-android-ghc`. `build-android.sh`
  is a real, honestly-flagged sketch, not a confirmed recipe — it now
  also has to work out how `cabal` itself cross-compiles a dependency
  closure, a question the original PoC never had to answer.
- Whether `jni_bridge.c`'s three real native methods
  (`DmmlBridge.render`/`.actions`/`.fire`) actually link and run once
  cross-compiled — the JNI name-mangling and stub-header path are
  followed carefully but unverified against a real cross build.
- Whether `MainActivity.kt`/`DmmlBridge.kt`/the Gradle files here
  actually produce a working APK, and whether the on-device app shows
  the SAME rendered snapshot + actions the host smoke test already
  confirmed — no Android device/emulator available to try.

## Layout

- `haskell/` — removed 2026-09-06. The real bridge now lives in
  `dmml-hs/src/DMML/JniBridge.hs`, alongside the interpreter it calls,
  per the JNI-vs-IPC dev-journal's own reasoning for why that's the
  right layer.
- `jni/jni_bridge.c` — the JNI side. `JNI_OnLoad` boots the RTS once;
  three native methods (`render`/`actions`/`fire`) on a
  `DmmlBridge` class call straight into `DMML.JniBridge`'s exported
  `dmml_render`/`dmml_actions`/`dmml_fire` symbols.
- `android/` — a minimal Gradle Android app. `DmmlBridge.kt` declares
  the `external fun`s; `MainActivity.kt` calls them against the same
  fixture the host smoke test already verified and shows the result in
  one `TextView`. Deliberately no CMake/ndk-build integration — the
  `.so` is built externally by `build-android.sh` and dropped into
  `app/src/main/jniLibs/<abi>/`, which Android Gradle Plugin packages
  automatically with zero extra config.
- `build-android.sh` — the real cross-compile steps, unverified (see
  above) — now cross-building a whole cabal package, not one file.

## Next real step

Get (or build, via `hatter`) a real `aarch64-linux-android-ghc` on a
machine that also has this repo's confirmed-working NDK, run
`build-android.sh`, fix whatever it gets wrong (something will,
especially the cabal-cross-compilation step this version newly
introduces), then `./gradlew assembleDebug` and confirm on a real
device or emulator that the app shows the same rendered snapshot +
actions `dev-journal/2026-09-06-real-jni-bridge-verified-on-host.md`
already confirmed on host GHC. Only once that's real is F1 actually
closed — the interpreter side is no longer in question; only the
cross-compilation and the JNI-specific parts are.

**F1 closed 2026-09-06** (`03d25f0`, real cross-compilation, real
Compose UI, verified on-device via emulator screenshot). **Next real
work — JGit-backed on-device persistence and the in-product authoring
loop — has a proposed starting structure, not yet built or agreed line
by line**: `written-world/dev-journal/2026-09-06-android-canonical-
repo-structure.md`. Read it before restructuring `android-poc/` or
adding Kotlin code — it proposes graduating the actual app (Kotlin/
Compose/Gradle) out of `android-poc/` into `written-world/android/`
while `JniBridge.hs`/`jni_bridge.c` stay here, and flags several real
open questions (binary-vendor-vs-rebuild for the `.so`, whether
`JniBridge.hs` needs new directory-reading entry points) as this
session's calls to make, not settled decisions.
