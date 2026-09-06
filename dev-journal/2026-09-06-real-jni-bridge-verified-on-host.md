# The real Android JNI bridge exists and is verified on host GHC

Direct follow-through on `dev-journal/2026-09-04-android-jni-vs-ipc.md`'s
recommendation (JNI, in-process, calling `dmml-hs`'s own library
functions directly) and Jason's own framing that this "should be very
simple" now that `world-browser-demo` had already proved the whole
materialize/render/act loop works with real content, zero LLM. It was:
the bridge module itself is small and clean. What was NOT simple, and is
the actual subject of this entry, is that this session ALSO had to stand
up a full local Android+Haskell toolchain from nothing — this repo is
running inside Android Studio on Windows, and neither GHC nor the NDK
was present before today.

## Toolchain, installed and confirmed this session

- **Android NDK 27.2.12479018**, via `sdkmanager` (first had to fetch
  `cmdline-tools` by hand — none was installed alongside the SDK).
  Confirmed: `aarch64-linux-android24-clang[.exe]` present under
  `%ANDROID_HOME%/ndk/27.2.12479018/toolchains/llvm/prebuilt/windows-x86_64/bin/`.
- **GHC 9.10.3 + cabal 3.16.1.0**, via `ghcup` (Windows install, non-
  interactive bootstrap). Confirmed: `ghc-9.10.3.exe --version` and
  `cabal.exe --version` both run.
- **Still NOT installed anywhere**: a GHC cross-compiler targeting
  `aarch64-linux-android` (the `hatter`-provisioned toolchain
  `android-poc/BOOTSTRAP.md` already flagged as the genuinely uncertain
  step). This session's NDK+host-GHC combination is necessary but not
  sufficient for a real cross-compile — see `android-poc/build-android.sh`'s
  own updated comments for exactly what's still missing there.

## What DMML.JniBridge is, and what it deliberately isn't

`dmml-hs/src/DMML/JniBridge.hs` exports three `foreign export ccall`
functions, calling the SAME library layer `app/RenderSnapshot.hs` and
`app/FireTransition.hs` already sit at, not a second wrapper around
either CLI's argv interface (the JNI-vs-IPC entry's own point):

- `dmml_render` — `DMML.Materialize.applyIdentifiedCommit` +
  `renderSnapshot`.
- `dmml_actions` — the same materialize step, then
  `DMML.Guard.availableTransitions` over a one-machine map.
- `dmml_fire` — the same materialize step, then
  `DMML.Fire.fireTransition` + `renderFiredCommit`.

Every one of the three wraps its entire body in a `Control.Exception.try
\@SomeException` backstop (`guardedRun`) and marshals ANY failure — a
parse error, a `FireError`, or a genuinely unexpected exception — to a
plain `"ERROR: ..."`-prefixed `CString`, never letting an exception
escape across the FFI boundary raw. This is the JNI-vs-IPC entry's
"required for the real bridge" mitigation, built in from the start
rather than retrofitted.

Deliberately narrower than the CLIs' own multi-file generality: exactly
ONE world commit and ONE machine per call, mirroring
`examples/world-browser-demo`'s own fixture shape rather than
`RenderSnapshot.hs`'s N-file list. Passing several commits/machines
across a JNI boundary needs a real array-marshaling decision
(`jobjectArray` of `jstring` on the JNI side) deliberately deferred
rather than guessed at with no real caller to design it against yet.

## A real bug this caught before it ever reached JNI

The first version of `materialize` used `DMML.Materialize.applyCommit`
(no provenance) — which meant `dmml_fire` could never actually fire ANY
transition with a `retract` effect (i.e. almost any real state
transition), because `DMML.Fire.fireTransition` correctly refuses to
cite a retraction against a fact with no real `StrongRef`
(`FireRetractNoProvenance`). Fixed the same way `app/FireTransition.hs`
already does it: `DMML.LocalIdentity.localFileRef` over the caller's own
source bytes (labeled `"android:worldSrc"` — `localFileRef`'s path
argument is just an opaque label, it never touches the filesystem, so
this is legitimate on the JNI side where there is no real file). Caught
by the host smoke test below, not by inspection — worth recording
because it's exactly the kind of gap that would have surfaced first as a
confusing on-device failure, much later, with a much slower feedback
loop.

## Verified, for real, on host GHC, in this environment

`dmml-hs/app/JniBridgeSmokeTest.hs` (`cabal run jni-bridge-smoke-test`)
calls the EXACT exported `CString` functions through real
`Foreign.C.String` marshaling — not their pure Haskell cores directly —
against a small self-contained fixture (`player/one` in `room/forge`,
one machine with a `work()` transition that both asserts and retracts a
fact). Real output, this session:

```
=== dmml_render ===
Declared predicates:
  relation location
  relation state

Facts:
  player/one . location = room/forge
  player/one . state = idle

=== dmml_actions (before firing) ===
machine/actions/work

=== dmml_fire (work) ===
commit android_fire
  declare relation state
  player/one `state` working
  consumes
    fact local:android:worldSrc#fnv1a64:cff13c6976ae0ea9
      player/one . state

=== error path: unknown transition ===
ERROR: FireNotDeclared

=== error path: malformed world source ===
ERROR: <surface>:1:1:
  |
1 | this is not dmml at all {{{
  | ^^^^^^
unexpected "this i"
expecting "commit"

all checks passed
```

Every check — render succeeds, actions correctly lists `work()` as
legal, fire produces a real re-parseable commit citing real provenance,
and both error paths return `"ERROR: ..."` rather than crashing —
passed. This is the whole materialize→render→act→fire loop, through the
exact FFI-shaped surface a JNI caller would use, proven end to end
before any JNI/Android code was even written.

## What's still genuinely unverified

Exactly what `android-poc/README.md`'s own updated "What this proves,
and what it doesn't" section says: `jni/jni_bridge.c` (rewritten to call
the three real `dmml_*` symbols instead of the old fake `hsGreet`),
`android/`'s `DmmlBridge.kt`/`MainActivity.kt` (rewritten to call
`DmmlBridge.render`/`.actions` against the same fixture the host smoke
test verified), and `build-android.sh` (rewritten to reflect that
cross-compiling now means building `dmml-hs`'s whole dependency closure
via cabal + a cross GHC, not one dependency-free file — a materially
bigger, still-unattempted lift) are all real source, carefully written
against documented JNI/cabal conventions, but NONE of it has been
cross-compiled, linked, or run — this machine still lacks the one piece
every one of these documents keeps naming as the actual blocker: a real
`aarch64-linux-android-ghc`.

## Addendum, same day: two host-side frontends, then a real Compose UI

Jason: "is this the right architecture for an Android native app?" —
asked after a quick Windows-only prototype (`app/TouchBrowser.hs`, a
hand-rolled localhost HTTP server built on the same primitives, driving
a "tap a link, no typing" UI in a browser). Real answer: no — an
embedded HTTP server is exactly the indirection this file's own JNI-vs-
IPC reasoning argues against. `TouchBrowser.hs`'s actual value was
proving the INTERACTION SHAPE (render + tappable action list + fire)
fast, without needing any Android toolchain — not a component meant to
ship. The right shape for the real app is native UI calling `DmmlBridge`
directly, no server in between.

That surfaced a REAL GAP the original single-commit bridge functions
couldn't cover: a multi-round UI (fire, see the new state, fire again)
needs to pass a GROWING history across the JNI boundary, and
`dmml_render`/`dmml_actions`/`dmml_fire` each take exactly one world
commit — a deliberate v1 scoping this module's own header already
disclosed, but a real limit the moment a caller needed more than one
fire. Fixed with `dmml_render_history`/`dmml_actions_history`/
`dmml_fire_history` — the history is a JSON array of world-commit
strings (oldest first), decoded with `Data.Aeson` (already a dependency)
rather than inventing a raw `jobjectArray`/`Ptr (Ptr CChar)` marshaling
scheme. Each history entry gets its OWN content-addressed provenance
ref, labeled by position (`android:history0`, `android:history1`, ...),
so a later fire's retract correctly cites an EARLIER fire's own fact,
not a fabricated shared one.

Verified on host GHC via `jni-bridge-smoke-test`'s extended history-mode
section: two real rounds (`work` then `rest`), each round re-checking
available actions and firing against the CORRECT accumulated state,
confirming the second fire's citation names `android:history1` (the
first fire), not the original world. One real bug caught in the
process — not in `DMML.JniBridge` itself, but in the test's own machine
fixture, which only declared `work()` and asserted `rest()` should be
offered after firing it; fixed by actually declaring `rest()`.

`android/`'s `GameScreen.kt` (new) wires these history functions to a
real Jetpack Compose screen — one growing `List<String>` in
`remember { mutableStateOf(...) }`, JSON-encoded via `org.json.JSONArray`
(Android framework, no new dependency) each recomposition, one `Button`
per currently-legal action, no text field anywhere. `build.gradle.kts`
now actually applies the Kotlin Android plugin and Kotlin 2.0's Compose
compiler plugin — genuinely missing before this session, an unverified
gap the original PoC's own README had already flagged under "whether
the Gradle files here actually produce a working APK."

At the time this was written, none of it had actually been compiled —
no Gradle wrapper existed in this project and no standalone Gradle
install existed on this machine either. Fixed later the same day; see
the next addendum.

## Addendum, same day: standing up real Gradle, and the actual first compile

Jason: "go ahead and set up Gradle." Real, not trivial — this surfaced
several more genuine gaps, each fixed by an actual error message, not
guessed at:

- **Android Studio's bundled JBR is JDK 25.0.3** — much newer than
  assumed. Gradle 8.7 (the obvious first choice) cannot even start its
  daemon on it (`FAILURE: ... 25.0.3` — Gradle's own JDK-too-new
  rejection). Fixed by using Gradle 9.7.1 (current stable as of this
  session) instead.
- Gradle 9.7.1 needs a matching AGP — this project's pinned AGP 8.5.0
  predates it entirely. Bumped to AGP 9.4.0 (current stable) and Kotlin
  2.4.10 to match.
- **`settings.gradle.kts` never declared `google()` as a plugin
  repository at all** — a real, disclosed gap from this PoC's original
  creation, never caught because it had never actually been run before.
  Without it, Gradle has nowhere to resolve `com.android.application`
  from. Added a proper `pluginManagement`/`dependencyResolutionManagement`
  block with `google()`/`mavenCentral()`/`gradlePluginPortal()`.
- **AGP 9.0+ has built-in Kotlin support and actively REJECTS the
  separate `org.jetbrains.kotlin.android` plugin** ("no longer required
  for Kotlin support since AGP 9.0") — removed it from both
  `build.gradle.kts` and `app/build.gradle.kts`, keeping
  `org.jetbrains.kotlin.plugin.compose` (still separate, still needed —
  built-in Kotlin support covers plain compilation, not the Compose
  compiler).
- That removal made `app/build.gradle.kts`'s `kotlinOptions { jvmTarget
  = "17" }` block dead (`Unresolved reference 'kotlinOptions'` — that
  DSL came from the plugin just removed). Deleted it; `compileOptions`'s
  `sourceCompatibility`/`targetCompatibility` (already `VERSION_17`)
  covers the same ground under built-in Kotlin support.
- **A real bug in `GameScreen.kt` itself**, caught only by actually
  compiling it: `import androidx.compose.foundation.layout.weight`
  resolved to the WRONG symbol (`Cannot access 'val
  RowColumnParentData?.weight: Float': it is internal in file`) — a
  hand-review had already caught one bad import
  (`androidx.compose.ui.Modifier`, see above) but missed this one.
  `ColumnScope.weight` is a scope MEMBER function, resolved
  automatically inside `Column { ... }`'s content lambda via its
  implicit `ColumnScope` receiver — it should never have been imported
  explicitly at all. Removing the import (not replacing it with a
  different one) was the fix.

**Confirmed for real, this session**: `./gradlew.bat :app:compileDebugKotlin`
— `BUILD SUCCESSFUL`. `GameScreen.kt`, `MainActivity.kt`, and
`DmmlBridge.kt` (Android-poc's ENTIRE Kotlin/Compose source) compile
clean against the real toolchain now standing on this machine: JDK
25.0.3 (Android Studio's own JBR) → Gradle 9.7.1 → AGP 9.4.0 → Kotlin
2.4.10 via AGP's built-in Kotlin support. First time any of this
project's Kotlin code has been compiled at all, not just written.

**Still unverified, same root cause as ever**: `compileDebugKotlin`
proves the Kotlin/Compose SOURCE is real and correct — it does not
attempt `assembleDebug` (packaging into an APK), which would fail
immediately on the missing `libdmmlbridge.so` in `jniLibs/`. That .so
still doesn't exist anywhere, on this machine or any other reachable
from this session — the cross-compiling `aarch64-linux-android-ghc`
remains the one piece every document in this file keeps naming as the
actual, unchanged blocker. Compiling the UI and cross-compiling the
interpreter into a loadable native library are two separate
unknowns, and only the first is resolved as of this addendum.
