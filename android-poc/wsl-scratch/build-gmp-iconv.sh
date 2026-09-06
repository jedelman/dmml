#!/usr/bin/env bash
set -euo pipefail
export PATH=/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

export CC=aarch64-linux-android-clang
export AR=llvm-ar
export RANLIB=llvm-ranlib
PREFIX=/home/edelmanja/android-libs

mkdir -p ~/native-build
cd ~/native-build

echo "=== GMP ==="
if [ ! -f gmp-6.3.0.tar.xz ]; then
  curl -fSL -o gmp-6.3.0.tar.xz https://gmplib.org/download/gmp/gmp-6.3.0.tar.xz
fi
rm -rf gmp-6.3.0
tar xf gmp-6.3.0.tar.xz
cd gmp-6.3.0
./configure --host=aarch64-linux-android --prefix="$PREFIX" --disable-shared --enable-static
make -j"$(nproc)"
make install
cd ..

echo "=== libiconv ==="
if [ ! -f libiconv-1.17.tar.gz ]; then
  curl -fSL -o libiconv-1.17.tar.gz https://ftp.gnu.org/pub/gnu/libiconv/libiconv-1.17.tar.gz
fi
rm -rf libiconv-1.17
tar xf libiconv-1.17.tar.gz
cd libiconv-1.17
./configure --host=aarch64-linux-android --prefix="$PREFIX" --disable-shared --enable-static
make -j"$(nproc)"
make install
cd ..

ls -la "$PREFIX/lib"
