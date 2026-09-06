#!/usr/bin/env bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
mkdir -p ~/wrapper-bin
cat > ~/wrapper-bin/aarch64-linux-android-clang <<'EOF'
#!/usr/bin/env bash
exec /home/edelmanja/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android-clang -I/home/edelmanja/libffi-android/include -L/home/edelmanja/libffi-android/lib "$@"
EOF
chmod +x ~/wrapper-bin/aarch64-linux-android-clang
cat ~/wrapper-bin/aarch64-linux-android-clang
