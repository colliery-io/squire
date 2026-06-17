plugins {
    alias(libs.plugins.kotlin.jvm)
}

// Pure-Kotlin/JVM offline engine for the Knight (parent) app — the privileged analog of `:core`.
// It serializes only `:sdk` types (already @Serializable), so it needs `Json` but not the
// kotlinx-serialization plugin. No Android, no okhttp — JVM-testable with fakes.
dependencies {
    api(project(":sdk"))
    api(libs.kotlinx.coroutines.core)
    implementation(libs.kotlinx.serialization.json)

    testImplementation(kotlin("test"))
    testImplementation(libs.kotlinx.coroutines.test)
}

tasks.test {
    useJUnitPlatform()
}

kotlin {
    jvmToolchain(17)
}
