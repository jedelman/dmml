#!/usr/bin/env bash
set -euo pipefail
export PATH=/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

CROSS_GHC=/data/data/com.termux/files/usr/bin/aarch64-linux-android-ghc

echo 'main = putStrLn "hi"' > ~/Smoke.hs

echo "=== compiling Smoke.hs with the cross-GHC ==="
bash "$CROSS_GHC" -c ~/Smoke.hs -o ~/Smoke.o
echo "=== file ~/Smoke.o ==="
file ~/Smoke.o
