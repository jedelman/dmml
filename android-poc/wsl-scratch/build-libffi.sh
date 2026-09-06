#!/usr/bin/env bash
set -euo pipefail
export PATH=/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

mkdir -p ~/libffi-build
cd ~/libffi-build
if [ ! -f libffi-3.4.6.tar.gz ]; then
  curl -fSL -o libffi-3.4.6.tar.gz https://github.com/libffi/libffi/releases/download/v3.4.6/libffi-3.4.6.tar.gz
fi
rm -rf libffi-3.4.6
tar xf libffi-3.4.6.tar.gz
cd libffi-3.4.6

export CC=aarch64-linux-android-clang
export AR=llvm-ar
export RANLIB=llvm-ranlib

./configure --host=aarch64-linux-android --prefix=/home/edelmanja/libffi-android --disable-shared --enable-static
make -j"$(nproc)"
make install

ls -la /home/edelmanja/libffi-android/include /home/edelmanja/libffi-android/lib
