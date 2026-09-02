# F1 — Android JNI bridge proof-of-concept

The last open item from `jedelman/dmml#1`. Jason: "go ahead and tackle
F1. keep it as simple as you can." This is that — the smallest real
thing that proves a Haskell function is callable across a JNI boundary
at all, following `hatter`'s documented shape (GHC NDK cross-compile to
a `.so`, a thin Kotlin activity that `System.loadLibrary`s it, a JNI C
bridge that boots the RTS and calls in) — see
`written-world`'s
`dev-journal/2026-09-02-platform-pivot-cli-android-filesystem-canonical.md`
for why this path was picked over `reflex-platform`/`obelisk` (moved
here from `written-world/android-poc/` per Jason's own call, 2026-09-02
— this repo is where `dmml-hs`, the interpreter this bridge exists to
carry, actually lives).

## What this proves, and what it doesn't

**No Android NDK, no Android SDK, no cross-compiling GHC, and no
device/emulator exist in the environment this was built in.** Rather
than pretend otherwise, this PoC is honestly split into what actually
got verified here and what still needs a real Android toolchain:

**Verified for real, on host GHC, in this environment:**
- `haskell/Bridge.hs` compiles clean (`ghc -c`) and `foreign export ccall
  hsGreet :: IO CString` produces exactly the C symbol and stub header a
  JNI bridge needs — confirmed by inspecting the compiled object
  (`nm Bridge.o` shows a real, callable `hsGreet` symbol, `T` not
  undefined) and the generated `Bridge_stub.h`.
- **The whole mechanism this PoC depends on — not just that it
  compiles** — was linked and *run*, natively (x86_64, not Android, but
  the same GHC FFI machinery JNI itself sits on top of): a small C
  `main()` calls `hs_init`, calls `hsGreet()`, prints the returned
  string, frees it, calls `hs_exit()`. Real output: `Haskell says: hello
  from the GHC RTS, via JNI`. This is the part most likely to hide a
  real bug (RTS init timing, string ownership/freeing across the FFI
  boundary, calling convention) — proven working, not just plausible.

**NOT verified — needs a real Android toolchain, on a machine that has
one:**
- Cross-compiling `Bridge.hs`/`jni_bridge.c` to `arm64-v8a` (or any
  Android ABI) at all. `build-android.sh` is a real, complete sketch of
  the steps (`hatter`'s own documented shape), but its exact toolchain
  binary names and flags are **not independently confirmed** — expect
  to need real fixes running it for the first time, the same way every
  other real mechanism built this session (sync-spike's hooks,
  checkpoint-per-commit) needed fixes once actually run. Don't treat
  this script as trustworthy until it's been run for real and corrected
  against whatever it actually gets wrong.
- Whether `MainActivity.kt`/`AndroidManifest.xml`/the Gradle files here
  actually produce a working APK — no Android SDK/`gradlew` available to
  try.
- Whether the loaded `.so` actually works on a real device/emulator —
  the JNI boundary itself (not just the Haskell FFI boundary already
  proven above) is unverified: `JNI_OnLoad`'s RTS-init timing relative
  to Android's own classloading, `System.loadLibrary`'s ABI resolution,
  and the JNI name-mangling in `jni_bridge.c`
  (`Java_org_writtenworld_androidpoc_MainActivity_greetFromHaskell`) are
  all standard, well-documented JNI conventions, followed here
  carefully, but "followed the convention correctly" and "confirmed
  working" are different claims — only the first is made here.

## Layout

- `haskell/Bridge.hs` — the Haskell side. One `foreign export ccall`
  function, deliberately not the real dmml-hs interpreter (see its own
  doc comment for why: proving the bridge mechanism is a different,
  prior question from proving the interpreter works once it's on the
  other side of it).
- `jni/jni_bridge.c` — the JNI side. `JNI_OnLoad` boots the RTS once;
  one native method calls straight into Haskell.
- `android/` — a minimal Gradle Android app (one `Activity`, one
  `TextView`) that loads the `.so` and calls it. Deliberately no
  CMake/ndk-build integration — the `.so` is built externally by
  `build-android.sh` and dropped into `app/src/main/jniLibs/<abi>/`,
  which Android Gradle Plugin packages automatically with zero extra
  config.
- `build-android.sh` — the real cross-compile steps, unverified (see
  above).

## Next real step

Run `build-android.sh` on a machine with a real Android NDK and a
`hatter`-provisioned cross GHC, fix whatever it gets wrong (something
will), then `./gradlew assembleDebug` and confirm on a real device or
emulator that the app launches and shows the Haskell-returned string.
Only once that's real is F1 actually closed — this PoC narrows what
that step still has to prove (the FFI mechanism itself is no longer in
question; only the cross-compilation and the JNI-specific parts are).
