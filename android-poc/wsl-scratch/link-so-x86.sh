#!/usr/bin/env bash
set -euo pipefail
NDK_BIN=/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin
export PATH="/home/edelmanja/wrapper-bin-x86:$NDK_BIN:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

CROSS_GHC=/data/data/com.termux/files/usr/bin/x86_64-linux-android-ghc
BUILD_DIR=/home/edelmanja/dmml-hs/dist-android-x86/build/x86_64-android/ghc-9.2.5/dmml-hs-0.1.0.0/build
NDK_SYSROOT=/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/sysroot
PKGDB=/home/edelmanja/dmml-hs/dist-android-x86/packagedb/ghc-9.2.5

mkdir -p ~/link-out-x86
cd ~/link-out-x86

echo "=== compiling jni_bridge.c ==="
"$CROSS_GHC" -c /mnt/c/Users/edelm/StudioProjects/dmml/android-poc/jni/jni_bridge.c -o jni_bridge.o \
  -optc-I"$BUILD_DIR" -optc-I"$NDK_SYSROOT/usr/include" -optc-I"$NDK_SYSROOT/usr/include/x86_64-linux-android"

echo "=== jni_bridge.o file type ==="
file jni_bridge.o

echo "=== linking libdmmlbridge.so ==="
"$CROSS_GHC" \
  -package-db "$HOME/.cabal/store/ghc-9.2.5/package.db" \
  -package-db "$PKGDB" \
  -package dmml-hs \
  -shared -o libdmmlbridge.so \
  jni_bridge.o \
  -optl-L/home/edelmanja/android-libs-x86/lib \
  -optl-Wl,-soname,libdmmlbridge.so \
  2>&1

echo "=== result ==="
file libdmmlbridge.so 2>&1

mkdir -p /mnt/c/Users/edelm/StudioProjects/dmml/android-poc/android/app/src/main/jniLibs/x86_64
cp libdmmlbridge.so /mnt/c/Users/edelm/StudioProjects/dmml/android-poc/android/app/src/main/jniLibs/x86_64/libdmmlbridge.so
echo "=== copied to jniLibs/x86_64/ ==="
ls -la /mnt/c/Users/edelm/StudioProjects/dmml/android-poc/android/app/src/main/jniLibs/x86_64/
