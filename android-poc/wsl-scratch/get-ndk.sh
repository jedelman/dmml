#!/usr/bin/env bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
mkdir -p ~/android-ndk
cd ~/android-ndk
if [ ! -f ndk.zip ]; then
  curl -fSL -o ndk.zip https://dl.google.com/android/repository/android-ndk-r27c-linux.zip
fi
ls -la
