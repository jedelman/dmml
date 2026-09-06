#!/usr/bin/env bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
mkdir -p /data/data/com.termux/files/usr
ln -sfn /home/edelmanja/ghc-target/lib /data/data/com.termux/files/usr/lib
ln -sfn /home/edelmanja/ghc-target/bin /data/data/com.termux/files/usr/bin
ls -la /data/data/com.termux/files/usr/
