# F1: Android JNI bridge proof-of-concept, honestly scoped

**Moved here from `written-world/dev-journal/` (same filename), 2026-09-02
— `android-poc/` itself moved from `written-world` to this repo the same
day, per Jason's own call ("move the android-poc work from written-world
to dmml"), since this is where `dmml-hs`, the interpreter the bridge
exists to carry, actually lives. Content below is unchanged from the
original, written when the work still lived in `written-world`.**

Jason: "go ahead and tackle F1. keep it as simple as you can." The
plan's own F1 (`android-poc/` this session): "an actual 'hello world'
Haskell-via-JNI proof-of-concept on Android, following `hatter`'s
demonstrated shape, before committing the whole interpreter to that
path."

## The real constraint that shaped the scope

No Android NDK, no Android SDK, no cross-compiling GHC (`ghcup` isn't
even installed), and no device/emulator exist in this session's
environment — confirmed by checking, not assumed. Disk was also already
tight (4GB free) after this session's own earlier endurance runs, which
ruled out trying to download an NDK (1-2GB+) speculatively. Given that,
attempting the full cross-compile-to-APK-to-device chain here would have
meant either quietly failing partway through, or claiming success on
something never actually run — both worse than being direct about what
a sandboxed dev environment with no mobile toolchain can and can't prove.

**The split this produced**: separate "does the FFI/bridge mechanism
itself work" (fully provable here, on host GHC) from "does cross-
compiling and packaging it for Android work" (not provable here at all,
needs a real toolchain). `android-poc/README.md`'s own "What this
proves, and what it doesn't" section is the authoritative, detailed
version of this; this entry is the narrative.

## What got proven for real

`android-poc/haskell/Bridge.hs`: one `foreign export ccall hsGreet ::
IO CString` function. Two real checks, not just "it should work":

1. `ghc -c Bridge.hs` compiles clean and produces a real `hsGreet` `T`
   (callable) symbol in the object file (confirmed via `nm`), plus the
   `Bridge_stub.h` a C caller needs — the FFI export mechanism itself,
   working.
2. **The whole round-trip, actually run**: a small C `main()` (`hs_init`
   → call `hsGreet()` → print → `free()` the C-allocated result →
   `hs_exit`), linked against the compiled Haskell object with host GHC,
   native x86_64. Real output: `Haskell says: hello from the GHC RTS,
   via JNI`. This is the part most likely to hide a real bug — RTS init
   timing, C-string ownership across the FFI boundary (`newCString`
   allocates via the C allocator specifically so a plain C `free()` is
   correct, not a leak or double-free) — and it's proven working, not
   just plausible-looking code.

This matters because it means the cross-compile step (Android NDK
target instead of host x86_64) is now *only* a toolchain problem, not
also an unverified correctness-of-approach problem. If a future NDK-
provisioned run of `build-android.sh` fails, the bug is almost certainly
in cross-compilation specifics, not in whether a Haskell function can
correctly hand a string back across a foreign-export boundary at all.

## What's real code but genuinely unverified

`android-poc/jni/jni_bridge.c` (JNI_OnLoad + one native method,
following standard JNI naming conventions carefully) and
`android-poc/android/` (a minimal Gradle app, one `Activity`/one
`TextView`, `.so` dropped into `jniLibs/<abi>/` rather than a custom
CMake/ndk-build integration — the simplest packaging path available)
are real, reviewable source, written to the documented pattern — but
"written correctly to the documented pattern" and "confirmed working"
are different claims, and only the first one is made. `build-android.sh`
sketches the real cross-compile steps but its exact toolchain binary
names/flags are explicitly flagged as unconfirmed in its own comments —
expect it to need real fixes on first actual use, the same way every
other real mechanism this session touched (sync-spike's hooks,
checkpoint-per-commit) needed fixes once actually run for the first
time, not zero.

## What's left, real and open

Run `build-android.sh` on a machine with a real NDK + `hatter`-
provisioned cross GHC, fix whatever it gets wrong, `./gradlew
assembleDebug`, confirm on a real device or emulator. Only that closes
F1 for real — this PoC's job was narrowing what that step still has to
prove, not replacing it.
