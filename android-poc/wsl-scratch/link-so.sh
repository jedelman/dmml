#!/usr/bin/env bash
set -euo pipefail
export PATH=/home/edelmanja/wrapper-bin:/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

CROSS_GHC=/data/data/com.termux/files/usr/bin/aarch64-linux-android-ghc
BUILD_DIR=/home/edelmanja/dmml-hs/dist-android/build/aarch64-android/ghc-9.2.5/dmml-hs-0.1.0.0/build
STUB_DIR=$BUILD_DIR/DMML
NDK_SYSROOT=/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/sysroot
PKGDB=/home/edelmanja/dmml-hs/dist-android/packagedb/ghc-9.2.5

mkdir -p ~/link-out
cd ~/link-out

echo "=== compiling jni_bridge.c ==="
"$CROSS_GHC" -c /mnt/c/Users/edelm/StudioProjects/dmml/android-poc/jni/jni_bridge.c -o jni_bridge.o \
  -optc-I"$BUILD_DIR" -optc-I"$NDK_SYSROOT/usr/include" -optc-I"$NDK_SYSROOT/usr/include/aarch64-linux-android"

echo "=== jni_bridge.o file type ==="
file jni_bridge.o

echo "=== linking libdmmlbridge.so ==="
"$CROSS_GHC" \
  -package-db "$HOME/.cabal/store/ghc-9.2.5/package.db" \
  -package-db "$PKGDB" \
  -package dmml-hs \
  -shared -o libdmmlbridge.so \
  jni_bridge.o \
  -optl-L/home/edelmanja/android-libs/lib \
  -optl-Wl,-soname,libdmmlbridge.so \
  2>&1

echo "=== result ==="
file libdmmlbridge.so 2>&1
