// Deliberately NO CMake/ndk-build integration -- the .so this app loads
// isn't built by Gradle's native toolchain at all, it's cross-compiled
// separately by ../../build-android.sh (a GHC-NDK cross-compile, an
// entirely different toolchain than anything Gradle's native plugins
// know how to drive) and dropped into src/main/jniLibs/<abi>/, which
// Android Gradle Plugin packages into the APK automatically with zero
// extra configuration -- the simplest integration point available,
// not a workaround.
//
// UPDATED 2026-09-06: Kotlin + Jetpack Compose added for the real UI
// layer (previously just one bare Activity/TextView -- the Kotlin
// plugin itself was actually missing before this change, an unverified
// gap this PoC's own README already flagged under "whether the Gradle
// files here actually produce a working APK"). No separate
// `org.jetbrains.kotlin.android` plugin -- AGP 9.0+ has built-in Kotlin
// support and rejects it outright (confirmed by actually running this
// project). `org.jetbrains.kotlin.plugin.compose` is still needed and
// still separate -- built-in Kotlin support covers plain compilation,
// not the Compose compiler.
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
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

    buildFeatures {
        compose = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // No separate `kotlinOptions {}` block -- that DSL extension came
    // from the `org.jetbrains.kotlin.android` plugin, which AGP 9's
    // built-in Kotlin support replaces outright (see this file's
    // `plugins` block comment). Built-in Kotlin support derives its JVM
    // target from `compileOptions` above -- confirmed by this exact
    // error surfacing here and being fixed by removing the block, not
    // by finding a replacement syntax for it.
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2024.06.00"))
    implementation("androidx.activity:activity-compose:1.9.0")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui-tooling-preview")
    debugImplementation("androidx.compose.ui:ui-tooling")
}
