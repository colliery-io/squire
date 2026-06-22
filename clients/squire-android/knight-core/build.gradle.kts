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

    // Gherkin/Cucumber acceptance suite (SQUIRE-T-0115): true `.feature` files driving the Knight
    // offline store on the JVM (no emulator), same scenario vocabulary as the api/Keep suites.
    testImplementation("io.cucumber:cucumber-java:7.18.1")
    testImplementation("io.cucumber:cucumber-junit-platform-engine:7.18.1")
    testImplementation("org.junit.platform:junit-platform-suite:1.10.2")
}

tasks.test {
    useJUnitPlatform()
    // Cucumber's JUnit Platform engine discovers features via the suite runner below.
    systemProperty("cucumber.junit-platform.naming-strategy", "long")
}

kotlin {
    jvmToolchain(17)
}
