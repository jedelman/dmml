#!/usr/bin/env bash
set -euo pipefail
NDK_BIN=/home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin

mkdir -p ~/wrapper-bin-x86
cat > ~/wrapper-bin-x86/x86_64-linux-android-clang <<EOF
#!/usr/bin/env bash
exec $NDK_BIN/x86_64-linux-android-clang -I/home/edelmanja/android-libs-x86/include -L/home/edelmanja/android-libs-x86/lib "\$@"
EOF
chmod +x ~/wrapper-bin-x86/x86_64-linux-android-clang

export PATH="/home/edelmanja/wrapper-bin-x86:$NDK_BIN:/home/edelmanja/.ghcup/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

CROSS_GHC=/data/data/com.termux/files/usr/bin/x86_64-linux-android-ghc
CROSS_GHC_PKG=/data/data/com.termux/files/usr/bin/x86_64-linux-android-ghc-pkg
CROSS_CC=x86_64-linux-android-clang

cd ~/dmml-hs
cabal build lib:dmml-hs \
  --with-compiler="$CROSS_GHC" \
  --with-hc-pkg="$CROSS_GHC_PKG" \
  --with-gcc="$CROSS_CC" \
  --builddir=dist-android-x86 \
  2>&1
