#!/usr/bin/env bash
set -euo pipefail
export JAVA_HOME="/mnt/c/Program Files/Android/Android Studio/jbr"
export ANDROID_HOME="/mnt/c/Users/edelm/AppData/Local/Android/Sdk"
export PATH="$JAVA_HOME/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
cd /mnt/c/Users/edelm/StudioProjects/dmml/android-poc/android
./gradlew assembleDebug 2>&1
echo "EXIT CODE: $?"
find . -iname "*.apk" 2>&1
