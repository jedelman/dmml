# The real Android cross-compile, linked, installed, and verified running

Direct continuation of `dev-journal/2026-09-06-real-jni-bridge-verified-on-host.md`
(which stopped at "the interpreter compiles and its FFI surface is verified
on host GHC — the actual Android cross-compile remains untouched"). Jason:
"go ahead and tackle the ghc cross compilation - shouldn't be too hard."
It was hard, in the specific and mostly-recoverable way this whole project's
dev-journal habitually is: several real, sequential blockers, each with a
real fix, no shortcuts taken. End state: `libdmmlbridge.so`, containing the
actual `dmml-hs` interpreter, cross-compiled for two Android ABIs, linked,
installed on a real (emulated) Android runtime, launched, and confirmed —
by screenshot, not just a clean log — rendering real DMML content via JNI.

## Toolchain, none of which existed on this machine before today

- **`hatter`** (the package every prior doc pointed to) turned out to
  require Nix — not a plain cross-GHC-on-PATH tool as assumed. Nix has no
  native Windows support; needs WSL2.
- **WSL2 was already installed** (Ubuntu-24.04, two distros registered) —
  no system-level changes needed, just entering it.
- **A real, prebuilt, x86_64-hosted GHC 9.2.5 cross-compiler** targeting
  `aarch64-unknown-linux-android`, from `MrAdityaAlok/ghc-cross-tools`
  (built for Termux's own package CI, genuinely rare to find as a usable
  binary rather than a build-from-source script). Its wrapper scripts hard-
  code Termux's install path (`/data/data/com.termux/files/usr`) — worked
  around by creating that exact path as a real directory tree on this
  disposable Linux VM, symlinked to the actual extracted location.
- Confirmed `dmml-hs` (2+ years newer, built against GHC 9.10.3) compiles
  unmodified on this GHC 9.2.5 cross-compiler's matching host-native
  version — no source incompatibility.
- The **same `ghc-cross-tools` release also ships an `x86_64-linux-android`
  target** — used later when the arm64 target turned out to be untestable
  on this host (see below).

## Cross-compiling the dependency closure — three real, fixable blockers

1. `fatal error: 'ffi.h' file not found` — the NDK doesn't bundle libffi
   headers. Built libffi 3.4.6 from source via the NDK's own cross clang.
2. `error: call to undeclared function 'getentropy'` — that Bionic libc
   function only exists from API 28+; the build was targeting API 24.
   Retargeted the clang symlink to API 28 rather than patch splitmix (an
   upstream dependency).
3. `unable to find library -lgmp` / `-liconv` — neither is bundled by the
   NDK either. Built GMP 6.3.0 and libiconv 1.17 from source the same way.

With all three real, `cabal build lib:dmml-hs --with-compiler=<cross-ghc>
--with-gcc=<cross-clang>` succeeded end to end — all 19 modules, including
`DMML.JniBridge`, produced real `ELF ... ARM aarch64` object files.

## Linking `libdmmlbridge.so` — two more real blockers, found only by actually trying to run it

Linking itself succeeded on the first real attempt (`ghc -shared -package
dmml-hs jni_bridge.o ...`, resolving the whole dependency closure via
`-package dmml-hs` against the cabal store's package db rather than hand-
listing every transitive package-id). Confirmed with `llvm-nm`: all 12
expected symbols present (`JNI_OnLoad`, the 6 real JNI entry points, the 6
underlying `dmml_*` exports).

That `.so` still didn't actually run. Two real `UnsatisfiedLinkError`s,
each caught only by installing and launching for real on a device:

- `library "libc++_shared.so" not found` — a standard, well-known NDK
  gotcha: the NDK's own C++ runtime shared library has to be bundled in
  `jniLibs/<abi>/` alongside any native library that links against it;
  Android doesn't resolve it automatically. Copied it in from the NDK
  sysroot for both ABIs.
- `cannot locate symbol "stg_SRT_1_info"`, then (after fixing that)
  `cannot locate symbol "ffi_call"` — GHC's `-shared` link mode does not,
  by default, pull in RTS/libffi object code that nothing in the directly-
  linked objects visibly calls (info-table/SRT machinery is referenced
  only indirectly, at runtime, not via a normal call the linker can see) —
  so `ld.lld` left those symbols "resolved externally," which on Android's
  `dlopen` (unlike a desktop dynamic linker with lazier binding
  conventions) fails hard at load time. Fixed by forcing `libHSrts.a` and
  `libffi.a` in wholesale: `-optl-Wl,--whole-archive -optl<path-to-.a>
  -optl-Wl,--no-whole-archive`. GMP was deliberately NOT given the same
  treatment — it links fine normally (real, direct symbol references from
  bignum code), and whole-archiving it too produced duplicate-symbol
  errors from GHC's own already-correct normal linking of it.

## The emulator itself refused the first target outright

`FATAL | Avd's CPU Architecture 'arm64' is not supported by the QEMU2
emulator on x86_64 host` — a real, current architectural constraint
(older emulator versions had software translation for this; the version
installed here, 37.1.11.0, does not). Not a bug to route around quickly.
Jason's call, given the options (build a second target / stop at object-
level proof / use a real device): build a second target. Repeated the
entire libffi/GMP/libiconv + cross-compile + link sequence for
`x86_64-linux-android`, hitting the exact same two link-time blockers
(`libc++_shared.so`, RTS/libffi whole-archiving) and fixing them the same
way — confirming those fixes are a property of this GHC/NDK linking shape
in general, not something specific to aarch64.

## Verified, for real, by screenshot

`system-images;android-28;google_apis;x86_64` installed, a real AVD
booted (`sys.boot_completed` = 1), `app-debug.apk` (both `.so` variants
bundled) installed via `adb install`, launched via `adb shell am start`.
Logcat: no crash, no `FATAL EXCEPTION`, no `UnsatisfiedLinkError`.
`ActivityManager: Displayed org.writtenworld.androidpoc/.MainActivity`.
Process alive. A real `adb shell screencap` pulled off the device shows
the actual rendered DMML content — `npc/smith` ("Tamsin," `role/oresmith`),
`room/forge` ("the smithy"), `player/one` idle — and a `work` button,
exactly matching what `GameScreen.kt` calling `DmmlBridge.renderHistory`/
`.actionsHistory` through JNI into the real, cross-compiled
`DMML.Materialize`/`DMML.Guard` should produce. Not a log line claiming
success — the actual interpreter's actual output, on screen, on a real
Android runtime.

## What's still open

- `libdmmlbridge.so` is unstripped (debug info included, ~75-94MB per
  ABI) — trivially reducible with `llvm-strip` if APK size matters for
  anything beyond this proof.
- The arm64-v8a build (the one that actually matters for a real device)
  is linked and bundled but has NOT been run on real arm64 hardware —
  only the x86_64 target has been launch-verified, because that's what
  this host's emulator can run at all. The arm64 `.so` went through the
  identical fix sequence (same blockers, same fixes, confirmed via `nm`)
  but "identical fix sequence, confirmed via nm" and "confirmed running"
  are different claims, same discipline this file applies everywhere
  else. A real arm64 device or a host that can emulate arm64 (Apple
  Silicon Mac, or an arm64 CI runner) would close this gap.
- None of this is committed. `git status` at time of writing shows main
  fully clean of these changes — this whole android-poc/dmml-hs/dev-
  journal set of edits is still sitting in the working tree only.
