#!/usr/bin/env bash
# Cross-compiles dmml-hs (via its DMML.JniBridge module) + jni_bridge.c
# into libdmmlbridge.so for a real Android device/emulator, following the
# `hatter` package's documented shape (GHC NDK cross-compile -> .so, JNI
# bridge, Kotlin `System.loadLibrary`) -- see written-world/dev-journal/
# 2026-09-02-platform-pivot-cli-android-filesystem-canonical.md for why
# this project picked that path over reflex-platform/obelisk.
#
# REWORKED 2026-09-06: the bridge is no longer one standalone file
# (android-poc/haskell/Bridge.hs, deleted) -- it's DMML.JniBridge, a real
# module INSIDE the dmml-hs cabal package, calling DMML.Materialize/
# DMML.Guard/DMML.Fire directly (dmml/dev-journal/2026-09-04-android-
# jni-vs-ipc.md's recommendation). That is a REAL, DISCLOSED increase in
# what this script has to get right: the original PoC cross-compiled one
# file with no dependencies at all (`aarch64-linux-android-ghc -c
# Bridge.hs`); this now needs dmml-hs's WHOLE dependency closure (aeson,
# megaparsec, parser-combinators, containers, text, bytestring,
# directory, filepath, unordered-containers -- see dmml-hs.cabal) built
# for aarch64-linux-android, which means either (a) a cross-compiling
# `cabal build` invocation using a cross GHC as its compiler (the
# realistic path -- cabal, not raw `ghc -c`, is what actually resolves
# and builds a dependency closure) or (b) hatter's own documented
# mechanism for this exact problem, if it has one beyond providing the
# cross GHC itself. NEITHER is verified here -- this script was updated
# on a machine that has a real Android NDK and a real host GHC/cabal
# (confirmed working: dmml-hs's whole library, including DMML.JniBridge,
# builds and its exported functions were verified end-to-end via
# `cabal run jni-bridge-smoke-test`, see dev-journal/2026-09-06-real-jni-
# bridge-verified-on-host.md) but STILL has no cross-compiling GHC
# targeting aarch64-linux-android and no device/emulator -- the exact
# same gap android-poc/README.md's original PoC disclosed, one layer
# further in now that the interpreter side behind the bridge is real.
#
# Prerequisites this script assumes but does not install or verify:
#   - Android NDK installed, $ANDROID_NDK_HOME set (this machine has one:
#     confirmed working aarch64-linux-android24-clang present).
#   - A GHC cross-compiler targeting aarch64-linux-android on PATH
#     (hatter's own provisioning mechanism, or built by hand against the
#     NDK above -- hatter's README is the actual authority on this step,
#     not this script) -- NOT present on this machine either; only a
#     HOST GHC (9.10.3, via ghcup) is confirmed here.
#   - That cross GHC's own `cabal` (or cabal configured with a
#     cross-compiling toolchain via a `cabal.project.local`/`~/.cabal/
#     config` `with-compiler`/`with-hc-pkg` stanza -- exact mechanism
#     depends on how the cross toolchain is provisioned, genuinely
#     uncertain without one in hand to try it against).
#   - $ANDROID_ABI set to the target ABI (default arm64-v8a below).

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DMML_HS_DIR="$(cd "$HERE/../dmml-hs" && pwd)"
ANDROID_ABI="${ANDROID_ABI:-arm64-v8a}"
API_LEVEL="${ANDROID_API_LEVEL:-24}"
OUT_DIR="$HERE/android/app/src/main/jniLibs/$ANDROID_ABI"

CROSS_GHC="${CROSS_GHC:-aarch64-linux-android-ghc}"
if ! command -v "$CROSS_GHC" >/dev/null 2>&1; then
  echo "build-android.sh: $CROSS_GHC not found on PATH -- this needs a real" >&2
  echo "  GHC-NDK cross toolchain (see hatter's own docs), not present here." >&2
  echo "  (Host GHC/cabal ARE confirmed working on this machine -- see" >&2
  echo "  dev-journal/2026-09-06-real-jni-bridge-verified-on-host.md --" >&2
  echo "  just not a cross-compiling one targeting Android.)" >&2
  exit 1
fi

if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  echo "build-android.sh: ANDROID_NDK_HOME must be set" >&2
  exit 1
fi

CLANG_TARGET="aarch64-linux-android${API_LEVEL}"
NDK_HOST_TAG="${NDK_HOST_TAG:-linux-x86_64}"  # e.g. windows-x86_64 on Windows -- see this machine's own confirmed layout under $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/
CC="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_HOST_TAG/bin/${CLANG_TARGET}-clang"
if [ ! -x "$CC" ]; then
  echo "build-android.sh: expected NDK clang at $CC -- adjust NDK_HOST_TAG for your host" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "[build-android] cross-building dmml-hs (incl. DMML.JniBridge) with $CROSS_GHC ..."
echo "[build-android] REAL, UNVERIFIED STEP: this needs cabal to resolve and build"
echo "[build-android] dmml-hs's full dependency closure using $CROSS_GHC as the compiler --"
echo "[build-android] exact invocation depends on how the cross toolchain wires cabal to it"
echo "[build-android] (a with-compiler stanza, a cabal.project.local, or hatter's own"
echo "[build-android] mechanism if it provides one) -- NOT hardcoded here since no real"
echo "[build-android] cross-cabal setup exists on this machine to verify the flags against."
(
  cd "$DMML_HS_DIR"
  cabal build lib:dmml-hs --with-compiler="$CROSS_GHC" --builddir="$WORK/dist-android"
)
# Object files this cabal build produced for DMML.JniBridge specifically --
# path depends on cabal's own dist-newstyle layout; adjust if this guess
# is wrong (genuinely likely, unverified against a real cross build).
JNIBRIDGE_O="$WORK/dist-android/build/aarch64-linux-android/ghc-*/dmml-hs-*/build/DMML/JniBridge.o"
STUB_H_DIR="$WORK/dist-android/build/aarch64-linux-android/ghc-*/dmml-hs-*/build"

echo "[build-android] compiling jni_bridge.c with $CC (needs the NDK's own JNI headers on the include path,"
echo "[build-android] plus DMML/JniBridge_stub.h from the cross build's own stub output dir above) ..."
"$CC" -I"$STUB_H_DIR" -c "$HERE/jni/jni_bridge.c" -o "$WORK/jni_bridge.o"

echo "[build-android] linking libdmmlbridge.so -- needs the cross GHC's own RTS libs AND"
echo "[build-android] the FULL dependency closure's object code on the link line, not just"
echo "[build-android] JniBridge.o + jni_bridge.o (the earlier hsGreet PoC had no dependencies"
echo "[build-android] at all, so this step is genuinely new complexity, not a copy-paste);"
echo "[build-android] letting $CROSS_GHC drive the link against the cabal-built package"
echo "[build-android] (rather than hand-assembling -L/-l flags for every transitive dependency,"
echo "[build-android] which this script would almost certainly get wrong) is the real next"
echo "[build-android] step to work out, e.g. via 'cabal build' emitting a linkable archive/"
echo "[build-android] object set cabal itself already knows how to link -- NOT attempted here."
echo "[build-android] placeholder link command, expected to need real fixes:"
echo "[build-android]   $CROSS_GHC -shared -o $OUT_DIR/libdmmlbridge.so $JNIBRIDGE_O $WORK/jni_bridge.o -optl-Wl,-soname,libdmmlbridge.so"
"$CROSS_GHC" -shared -o "$OUT_DIR/libdmmlbridge.so" $JNIBRIDGE_O "$WORK/jni_bridge.o" -optl-Wl,-soname,libdmmlbridge.so

echo "[build-android] wrote $OUT_DIR/libdmmlbridge.so"
echo "[build-android] next: cd $HERE/android && ./gradlew assembleDebug (needs the Android SDK,"
echo "[build-android] confirmed present on this machine, cmdline-tools + platform android-37.0)"
echo "[build-android] then install/run on a real device or emulator and confirm MainActivity"
echo "[build-android] shows the same rendered snapshot + actions dev-journal/2026-09-06-real-"
echo "[build-android] jni-bridge-verified-on-host.md already confirmed on host GHC."
