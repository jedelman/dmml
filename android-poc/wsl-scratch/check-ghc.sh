#!/usr/bin/env bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
GHC=/home/edelmanja/ghc-cross/lib/ghc-9.2.5/bin/ghc
TOPDIR=/home/edelmanja/ghc-cross/lib/ghc-9.2.5
"$GHC" -B"$TOPDIR" --info
