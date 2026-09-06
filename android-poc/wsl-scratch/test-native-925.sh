#!/usr/bin/env bash
set -euo pipefail
export PATH=/home/edelmanja/.ghcup/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
cabal update
cd ~/dmml-hs
cabal build lib:dmml-hs
