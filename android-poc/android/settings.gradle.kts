// pluginManagement/dependencyResolutionManagement blocks added 2026-09-06:
// this project's settings.gradle.kts never had them -- a real, disclosed
// gap from the original PoC (never actually run against a real Gradle
// before this session). Without `google()` here, Gradle has nowhere to
// resolve the Android Gradle Plugin (com.android.application) from at
// all -- every standard Android Studio-generated project has this,
// it was simply missing here.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "dmml-android-poc"
include(":app")
