#!/usr/bin/env bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
BUILD_DIR=/home/edelmanja/dmml-hs/dist-android/build/aarch64-android/ghc-9.2.5/dmml-hs-0.1.0.0/build

echo "=== stub header ==="
find "$BUILD_DIR" -iname "JniBridge_stub.h" 2>&1

echo "=== inplace package conf dir ==="
find /home/edelmanja/dmml-hs/dist-android -iname "package.conf.inplace" -maxdepth 6 2>&1

echo "=== any .a archive for dmml-hs ==="
find /home/edelmanja/dmml-hs/dist-android -iname "*.a" 2>&1

echo "=== ghc-pkg list (cross) for dmml-hs ==="
/data/data/com.termux/files/usr/bin/aarch64-linux-android-ghc-pkg --package-db=/home/edelmanja/dmml-hs/dist-android/build/aarch64-android/ghc-9.2.5/dmml-hs-0.1.0.0/package.conf.inplace list 2>&1 | head -20

echo "=== NDK jni.h ==="
find /home/edelmanja/android-ndk -iname "jni.h" 2>&1

echo "=== HsFFI.h location ==="
find /home/edelmanja/ghc-target -iname "HsFFI.h" 2>&1
