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
        // 28, not 24: dev-journal/2026-09-06-android-cross-compile-
        // verified-on-device.md's own real fix -- libdmmlbridge.so was
        // built against API 28 (getentropy() doesn't exist in Bionic
        // below it, and splitmix calls it unconditionally). A real API
        // 24-27 device would install this app fine and then fail to
        // dlopen the native library at runtime -- minSdk must match
        // what the .so actually needs, not what an earlier, since-
        // superseded build targeted.
        minSdk = 28
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

    // JGit for browsing's git-sync half (dmml/dev-journal/2026-09-07-
    // android-jgit-sync-spec.md) -- pure-JVM, no native code, no NDK/
    // cross-compile involvement, unlike everything DMML.JniBridge
    // itself needed. Real, disclosed unverified item: JGit's Android
    // compatibility is well-established by OTHER projects (Gerrit's own
    // tooling, MGit), not independently confirmed against this exact
    // toolchain (AGP 9.4.0 / Kotlin 2.4.10 / minSdk 24) until it's
    // actually built.
    implementation("org.eclipse.jgit:org.eclipse.jgit:6.10.0.202406032230-r")

    // Explicit, not relied-on-transitively -- WorldRepository's suspend
    // functions need a real coroutines dependency, not an assumption
    // that activity-compose happens to pull one in.
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")

    // OkHttp for DMML.Http's 2026-09-08 rewrite off java.net.http.HttpClient
    // (confirmed absent on Android/ART on any API level -- see
    // dmml-hs/src/DMML/Http.hs's own module haddock). Real, disclosed gap
    // this dependency fixes: it was verified on the desktop CLI's own
    // manually-supplied classpath but NOT added here until a real
    // on-device NativeBridge.atprotoResolve call crashed the whole ART
    // runtime with `JNI DETECTED ERROR ... ClassNotFoundException:
    // okhttp3.Request$Builder` -- proving the gap for real rather than
    // assuming Gradle would somehow pull it in. okio + kotlin-stdlib come
    // transitively (this is what "just a Gradle dependency" buys over the
    // desktop CLI's classpath, which has to list all three by hand).
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
}
