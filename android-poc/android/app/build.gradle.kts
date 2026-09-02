// Deliberately NO CMake/ndk-build integration -- the .so this app loads
// isn't built by Gradle's native toolchain at all, it's cross-compiled
// separately by ../../build-android.sh (a GHC-NDK cross-compile, an
// entirely different toolchain than anything Gradle's native plugins
// know how to drive) and dropped into src/main/jniLibs/<abi>/, which
// Android Gradle Plugin packages into the APK automatically with zero
// extra configuration -- the simplest integration point available,
// not a workaround.
plugins {
    id("com.android.application")
}

android {
    namespace = "org.writtenworld.androidpoc"
    compileSdk = 34

    defaultConfig {
        applicationId = "org.writtenworld.androidpoc"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "0.1"
    }
}
