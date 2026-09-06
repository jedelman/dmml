#!/usr/bin/env bash
set -euo pipefail
NDK_BIN=/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin
export PATH="$NDK_BIN:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

echo "=== clang shim ==="
cd "$NDK_BIN"
ln -sf x86_64-linux-android28-clang x86_64-linux-android-clang
ln -sf x86_64-linux-android28-clang++ x86_64-linux-android-clang++

export CC=x86_64-linux-android-clang
export AR=llvm-ar
export RANLIB=llvm-ranlib
PREFIX=/home/edelmanja/android-libs-x86

mkdir -p ~/native-build-x86
cd ~/native-build-x86

echo "=== libffi ==="
rm -rf libffi-3.4.6
tar xf ~/libffi-build/libffi-3.4.6.tar.gz
cd libffi-3.4.6
./configure --host=x86_64-linux-android --prefix="$PREFIX" --disable-shared --enable-static
make -j"$(nproc)"
make install
cd ..

echo "=== GMP ==="
rm -rf gmp-6.3.0
tar xf ~/native-build/gmp-6.3.0.tar.xz
cd gmp-6.3.0
./configure --host=x86_64-linux-android --prefix="$PREFIX" --disable-shared --enable-static
make -j"$(nproc)"
make install
cd ..

echo "=== libiconv ==="
rm -rf libiconv-1.17
tar xf ~/native-build/libiconv-1.17.tar.gz
cd libiconv-1.17
./configure --host=x86_64-linux-android --prefix="$PREFIX" --disable-shared --enable-static
make -j"$(nproc)"
make install
cd ..

ls -la "$PREFIX/include" "$PREFIX/lib"
