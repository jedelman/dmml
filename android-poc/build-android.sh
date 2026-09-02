#!/usr/bin/env bash
# Cross-compiles Bridge.hs + jni_bridge.c into libdmmlbridge.so for a
# real Android device/emulator, following the `hatter` package's
# documented shape (GHC NDK cross-compile -> .so, JNI bridge, Kotlin
# `System.loadLibrary`) -- see written-world/dev-journal/2026-09-02-
# platform-pivot-cli-android-filesystem-canonical.md for why this
# project picked that path over reflex-platform/obelisk.
#
# NOT RUNNABLE IN THE ENVIRONMENT THIS SCRIPT WAS WRITTEN IN: no
# Android NDK, no cross-compiling GHC, and no device/emulator were
# available there -- see android-poc/README.md's own "What this proves,
# and what it doesn't" section before trusting anything below. The
# exact toolchain binary names/flags here are a best-effort sketch of
# the documented shape, NOT independently verified against a real
# `hatter`-provisioned toolchain -- confirm against hatter's own docs
# (https://hackage.haskell.org/package/hatter) before running this for
# real, and expect to need real fixes, the same way every other real
# mechanism in this project's sync-spike/entropy-sidecar/checkpoint work
# needed fixes once actually run.
#
# Prerequisites this script assumes but does not install or verify:
#   - Android NDK installed, $ANDROID_NDK_HOME set.
#   - A GHC cross-compiler targeting aarch64-linux-android on PATH
#     (hatter's own provisioning mechanism, or built by hand against
#     the NDK above -- hatter's README is the actual authority on this
#     step, not this script).
#   - $ANDROID_ABI set to the target ABI (default arm64-v8a below).

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANDROID_ABI="${ANDROID_ABI:-arm64-v8a}"
API_LEVEL="${ANDROID_API_LEVEL:-24}"
OUT_DIR="$HERE/android/app/src/main/jniLibs/$ANDROID_ABI"

CROSS_GHC="${CROSS_GHC:-aarch64-linux-android-ghc}"
if ! command -v "$CROSS_GHC" >/dev/null 2>&1; then
  echo "build-android.sh: $CROSS_GHC not found on PATH -- this needs a real" >&2
  echo "  GHC-NDK cross toolchain (see hatter's own docs), not present here." >&2
  exit 1
fi

if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  echo "build-android.sh: ANDROID_NDK_HOME must be set" >&2
  exit 1
fi

CLANG_TARGET="aarch64-linux-android${API_LEVEL}"
CC="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/${CLANG_TARGET}-clang"
if [ ! -x "$CC" ]; then
  echo "build-android.sh: expected NDK clang at $CC -- adjust for your NDK layout/host" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "[build-android] compiling Bridge.hs with $CROSS_GHC ..."
"$CROSS_GHC" -c "$HERE/haskell/Bridge.hs" -o "$WORK/Bridge.o" -hidir "$WORK" -stubdir "$WORK"

echo "[build-android] compiling jni_bridge.c with $CC (needs the NDK's own JNI headers on the include path) ..."
"$CC" -I"$WORK" -c "$HERE/jni/jni_bridge.c" -o "$WORK/jni_bridge.o"

echo "[build-android] linking libdmmlbridge.so -- needs the cross GHC's own RTS libs on the link line;"
echo "[build-android] exact -L/-l flags depend on the toolchain's layout, not hardcoded here on purpose"
echo "[build-android] (a real run should let $CROSS_GHC drive the final link, e.g.:"
echo "[build-android]   $CROSS_GHC -shared -o $OUT_DIR/libdmmlbridge.so $WORK/Bridge.o $WORK/jni_bridge.o -optl-Wl,-soname,libdmmlbridge.so"
echo "[build-android] rather than hand-assembling C link flags this script would get wrong)."
"$CROSS_GHC" -shared -o "$OUT_DIR/libdmmlbridge.so" "$WORK/Bridge.o" "$WORK/jni_bridge.o" -optl-Wl,-soname,libdmmlbridge.so

echo "[build-android] wrote $OUT_DIR/libdmmlbridge.so"
echo "[build-android] next: cd $HERE/android && ./gradlew assembleDebug (needs the Android SDK, also not verified here)"
