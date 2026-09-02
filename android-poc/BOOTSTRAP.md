# Bootstrap: local Android + Haskell dev environment

For whoever picks up F1 next — a different harness/model, on Jason's
own laptop, per `dev-journal/2026-09-02-f1-android-handoff-to-laptop-
session.md`. Read that entry and `README.md` first; this file is just
the concrete "how do I get a machine that can actually build and run
this" steps neither of those needed to spell out.

**Read this whole file before running anything.** Every step notes what
it's confident about vs. what needs confirming against upstream docs on
your machine — the cloud environment that wrote this file had no
Android/NDK/cross-GHC toolchain to verify any of it against directly
(see `README.md`'s own "What this proves, and what it doesn't"). Treat
the uncertain steps as a real starting point, not gospel — expect to fix
things, the same way every other real mechanism in this repo needed
fixing once actually run for the first time (`sync-spike`'s hooks,
checkpoint-per-commit — see those dev-journal entries for the pattern).

## 0. What you're building toward

`build-android.sh` cross-compiles `haskell/Bridge.hs` + `jni/
jni_bridge.c` into `android/app/src/main/jniLibs/<abi>/libdmmlbridge.so`,
then `android/`'s Gradle project packages that into an APK. Confirming
this "done" means: the APK installs on a device/emulator, launches, and
shows the string `hello from the GHC RTS, via JNI` on screen — that
string coming from the Haskell side, not hardcoded anywhere in the
Kotlin/Java layer (check `MainActivity.kt` if you want to confirm this
directly — it really does come from `greetFromHaskell()`, a native
call).

## 1. Android SDK + NDK

Standard, well-documented territory — this part of the setup isn't in
question, unlike the cross-GHC step below.

- Install Android Studio (any recent version) OR just the command-line
  tools if you'd rather not run the IDE — either gives you `sdkmanager`.
- Via `sdkmanager` (or Android Studio's SDK Manager UI), install:
  - A platform matching `compileSdk`/`targetSdk` in
    `android/app/build.gradle.kts` (currently 34) — `sdkmanager
    "platforms;android-34"`.
  - The NDK — `sdkmanager --install "ndk;27.0.12077973"` (or whatever
    the current LTS-ish NDK version is; check `sdkmanager --list` — the
    exact version isn't load-bearing for this PoC, just needs to be
    recent enough that `aarch64-linux-android24-clang` exists in it,
    see step 3).
- Set `ANDROID_NDK_HOME` to wherever that installed
  (`$ANDROID_SDK_ROOT/ndk/<version>/` is the usual layout).
- Set `ANDROID_HOME`/`ANDROID_SDK_ROOT` per the normal Android tooling
  convention (Gradle's Android plugin needs this to find the SDK at
  all).

Verify: `ls "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt"` should show a
`<host-os>-x86_64` (or similar) directory containing `bin/` with clang
binaries named like `aarch64-linux-android24-clang`.

## 2. Host GHC (for editing/typechecking, not the cross-build itself)

```sh
curl --proto '=https' --tlsv1.2 -sSf https://get-ghcup.haskell.org | sh
ghcup install ghc     # any recent 9.x is fine
ghcup install cabal
```

Sanity check this matches what the cloud environment already confirmed
works (`Bridge.hs` compiles clean and the FFI round-trip runs — see
`dev-journal/2026-09-02-f1-android-jni-bridge-poc.md`):

```sh
cd android-poc/haskell
ghc -c Bridge.hs -o /tmp/Bridge.o
nm /tmp/Bridge.o | grep hsGreet   # should show a T (defined/callable) symbol, not U (undefined)
```

## 3. Cross-compiling GHC for `aarch64-linux-android` — the genuinely uncertain part

This is the one piece of this bootstrap that could NOT be verified
anywhere upstream of your machine — no cross-GHC toolchain existed in
the environment that wrote `build-android.sh`. Two real paths, in the
order this project's own research (`dev-journal/2026-09-02-platform-
pivot-cli-android-filesystem-canonical.md`) rated them:

1. **`hatter`** (Hackage: <https://hackage.haskell.org/package/hatter>)
   — the package this whole platform pivot was researched against.
   **Follow hatter's own README for the actual install/provisioning
   commands** — don't trust a paraphrase here to be current or exact.
   The expected shape, per that earlier research: it provisions (or
   documents how to provision) an `aarch64-linux-android-ghc` on `PATH`
   that `build-android.sh` already expects, targeting the NDK from step
   1.
2. **Hand-building a cross GHC** against the NDK yourself, if `hatter`
   turns out stale or doesn't fit your GHC/NDK version combination —
   more work, a real fallback, not the first thing to reach for.

Once you have a working `aarch64-linux-android-ghc` (however you got
it), verify it directly before trusting `build-android.sh` with it:

```sh
echo 'main = putStrLn "cross-compile smoke test"' > /tmp/smoke.hs
aarch64-linux-android-ghc /tmp/smoke.hs -o /tmp/smoke
file /tmp/smoke   # should say ELF ... ARM aarch64, not the host's own arch
```

If that smoke test doesn't produce a real ARM64 ELF binary, nothing
downstream will work either — fix this step before touching
`build-android.sh` at all.

## 4. Build the `.so` and the APK

```sh
cd android-poc
export ANDROID_NDK_HOME=...     # from step 1
export ANDROID_ABI=arm64-v8a    # matches most real devices/emulators
./build-android.sh
```

Read `build-android.sh`'s own comments as you go — it says up front
which parts (the final link step's exact flags especially) are a
best-effort sketch, not confirmed. If it fails, that's expected on a
first real run; fix it in place and consider updating this bootstrap
file's own notes once you know what was actually wrong, so the next
handoff doesn't rediscover the same fix.

```sh
cd android
./gradlew assembleDebug    # or installDebug with a device/emulator already connected
```

## 5. Confirm on a real device or emulator

```sh
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n org.writtenworld.androidpoc/.MainActivity
```

Look at the running app. It should show exactly `hello from the GHC
RTS, via JNI`. If it crashes or shows nothing, `adb logcat` is the
first place to look — a `UnsatisfiedLinkError` means the `.so` didn't
load (check `jniLibs/<abi>/` actually matches the device's real ABI);
a crash inside the native call likely means the RTS init in
`jni_bridge.c`'s `JNI_OnLoad` needs adjusting for however Android's
classloader timing differs from the plain C `main()` this was proven
against on the cloud side.

## 6. Once this actually works

Update `android-poc/README.md`'s "What this proves, and what it
doesn't" section to move the newly-confirmed items out of "NOT
verified" — and note there, plainly, whatever `build-android.sh` or
this file got wrong the first time, the same way this whole repo's
other dev-journal entries record real fixes rather than silently
patching around them. Then close the loop on `jedelman/dmml#1` — F1 is
the last open item on that tracking issue.
