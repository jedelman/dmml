#!/usr/bin/env bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
cd ~/android-ndk
unzip -q ndk.zip
ls ~/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin/ | grep -E "aarch64-linux-android|ld.lld|llvm-ar$|llvm-ranlib$"
