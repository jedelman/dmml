#!/usr/bin/env bash
set -euo pipefail
NDK_BIN=/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin
export PATH="/home/edelmanja/wrapper-bin-x86:$NDK_BIN:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

CROSS_GHC=/data/data/com.termux/files/usr/bin/x86_64-linux-android-ghc
PKGDB=/home/edelmanja/dmml-hs/dist-android-x86/packagedb/ghc-9.2.5

cd ~/link-out-x86

RTS_A=/home/edelmanja/ghc-target/lib/x86_64-linux-android-ghc-9.2.5/rts/libHSrts.a
FFI_A=/home/edelmanja/android-libs-x86/lib/libffi.a
GMP_A=/home/edelmanja/android-libs-x86/lib/libgmp.a
ICONV_A=/home/edelmanja/android-libs-x86/lib/libiconv.a

echo "=== relinking with --whole-archive on RTS + all C static libs ==="
"$CROSS_GHC" \
  -package-db "$HOME/.cabal/store/ghc-9.2.5/package.db" \
  -package-db "$PKGDB" \
  -package dmml-hs \
  -shared -o libdmmlbridge.so \
  jni_bridge.o \
  -optl-L/home/edelmanja/android-libs-x86/lib \
  -optl-Wl,--whole-archive -optl"$RTS_A" -optl"$FFI_A" -optl-Wl,--no-whole-archive \
  -optl-Wl,-soname,libdmmlbridge.so \
  2>&1

echo "=== checking symbol presence ==="
file libdmmlbridge.so
"$NDK_BIN/llvm-nm" -D libdmmlbridge.so 2>&1 | grep -c "stg_SRT_1_info" || echo "NOT FOUND as dynamic symbol"
"$NDK_BIN/llvm-nm" libdmmlbridge.so 2>&1 | grep "stg_SRT_1_info" || echo "NOT FOUND at all"
