#!/usr/bin/env bash
set -euo pipefail
export PATH=/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

echo "=== downloading x86_64 cross-GHC packages ==="
mkdir -p ~/ghc-target-x86-dl
cd ~/ghc-target-x86-dl
for f in ghc-cross-bin-9.2.5-x86_64.tar.xz ghc-9.2.5-x86_64.tar.xz; do
  if [ ! -f "$f" ]; then
    curl -fSL -o "$f" "https://github.com/MrAdityaAlok/ghc-cross-tools/releases/download/ghc-v9.2.5/$f"
  fi
done
tar xf ghc-cross-bin-9.2.5-x86_64.tar.xz -C ~/ghc-target-x86-dl/cross-bin --one-top-level 2>/dev/null || (mkdir -p cross-bin && tar xf ghc-cross-bin-9.2.5-x86_64.tar.xz -C cross-bin)
mkdir -p target
tar xf ghc-9.2.5-x86_64.tar.xz -C target

echo "=== merging into the same fake Termux tree ==="
mkdir -p ~/ghc-target/bin ~/ghc-target/lib
cp -rn ~/ghc-target-x86-dl/target/bin/* ~/ghc-target/bin/ 2>&1 || true
cp -rn ~/ghc-target-x86-dl/target/lib/* ~/ghc-target/lib/ 2>&1 || true

find ~/ghc-target/bin -iname "*x86_64-linux-android-ghc*"
find ~/ghc-target/lib -maxdepth 1 -iname "*x86_64*"

echo "=== fixing baked-in Termux paths for x86_64 packages ==="
grep -rl "/data/data/com.termux/files/usr/lib/x86_64-linux-android-ghc-9.2.5" ~/ghc-target/lib/package.conf.d/ 2>&1 | wc -l

echo "=== x86_64-linux-android-clang check ==="
NDK_BIN=~/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin
ls "$NDK_BIN" | grep "^x86_64-linux-android2[0-9]-clang$" | head -5
