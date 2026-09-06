#!/usr/bin/env bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ln -sf /bin/bash /data/data/com.termux/files/usr/bin/sh
/data/data/com.termux/files/usr/bin/aarch64-linux-android-ghc --version
