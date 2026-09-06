#!/usr/bin/env bash
set -euo pipefail
export PATH=/home/edelmanja/wrapper-bin:/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin:/home/edelmanja/.ghcup/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

CROSS_GHC=/data/data/com.termux/files/usr/bin/aarch64-linux-android-ghc
CROSS_GHC_PKG=/data/data/com.termux/files/usr/bin/aarch64-linux-android-ghc-pkg
CROSS_CC=aarch64-linux-android-clang

cd ~/dmml-hs
cabal build lib:dmml-hs \
  --with-compiler="$CROSS_GHC" \
  --with-hc-pkg="$CROSS_GHC_PKG" \
  --with-gcc="$CROSS_CC" \
  --builddir=dist-android \
  2>&1
