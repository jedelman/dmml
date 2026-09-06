#!/usr/bin/env bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
mkdir -p ~/ghc-target
cd ~/ghc-target
curl -fSL -o ghc-9.2.5-aarch64.tar.xz https://github.com/MrAdityaAlok/ghc-cross-tools/releases/download/ghc-v9.2.5/ghc-9.2.5-aarch64.tar.xz
tar xf ghc-9.2.5-aarch64.tar.xz
find . -maxdepth 4 -iname "package.conf.d" 2>&1
find . -maxdepth 3 2>&1
