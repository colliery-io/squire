plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.openapi.generator)
}

val generatedDir = layout.buildDirectory.dir("generated/sdk")

openApiGenerate {
    generatorName.set("kotlin")
    library.set("jvm-okhttp4")
    inputSpec.set("$rootDir/../../crates/api/openapi.json")
    outputDir.set(generatedDir.get().asFile.path)
    packageName.set("com.squire.sdk")
    apiPackage.set("com.squire.sdk.api")
    modelPackage.set("com.squire.sdk.model")
    additionalProperties.set(
        mapOf(
            "serializationLibrary" to "kotlinx_serialization",
        ),
    )
}

sourceSets["main"].kotlin.srcDir(generatedDir.map { it.dir("src/main/kotlin") })

tasks.named("compileKotlin") {
    dependsOn(tasks.named("openApiGenerate"))
}

dependencies {
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.kotlinx.coroutines.core)
    implementation(libs.okhttp)
    implementation(libs.okhttp.logging.interceptor)

    // SDK decode round-trip test (SQUIRE-T-0033): proves the generated ClaimState/RedemptionState/
    // LockReason data classes parse the real server JSON.
    testImplementation(kotlin("test"))
    testImplementation(libs.kotlinx.serialization.json)
}

// The generated SDK sources must exist before the test source set compiles against them.
tasks.named("compileTestKotlin") {
    dependsOn(tasks.named("openApiGenerate"))
}

tasks.test {
    useJUnitPlatform()
}

kotlin {
    jvmToolchain(17)
}
