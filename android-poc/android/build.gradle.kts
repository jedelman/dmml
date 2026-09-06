// Versions bumped 2026-09-06 to match this machine's real toolchain --
// Android Studio's bundled JBR turned out to be JDK 25.0.3 (found the
// hard way: Gradle 8.7 cannot run on it at all), which needs Gradle
// 9.7.1 (the current stable release as of this session), which in turn
// needs an AGP/Kotlin pairing that actually supports Gradle 9.x -- AGP
// 8.5.0/Kotlin 2.0.20 (this file's original pins) predate all of that.
//
// No separate `org.jetbrains.kotlin.android` plugin here: AGP 9.0+ has
// built-in Kotlin support and actively REJECTS that plugin now
// (confirmed by running this project for real -- "the
// 'org.jetbrains.kotlin.android' plugin is no longer required for
// Kotlin support since AGP 9.0"). `org.jetbrains.kotlin.plugin.compose`
// is still needed and still separate -- built-in Kotlin support covers
// plain Kotlin compilation, not the Compose compiler.
plugins {
    id("com.android.application") version "9.4.0" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.4.10" apply false
}
