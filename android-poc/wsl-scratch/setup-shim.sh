#!/usr/bin/env bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
NDK_BIN=~/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin
cd "$NDK_BIN"
# API 24 lacks getentropy() (added in Bionic at API 28) -- splitmix's
# cbits-unix/init.c calls it unconditionally. Retargeting to API 28
# rather than patching an upstream dependency.
ln -sf aarch64-linux-android28-clang aarch64-linux-android-clang
ln -sf aarch64-linux-android28-clang++ aarch64-linux-android-clang++
ls -la aarch64-linux-android-clang aarch64-linux-android-clang++
