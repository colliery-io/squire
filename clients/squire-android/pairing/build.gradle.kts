plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.kotlin.compose)
}

// Shared device-pairing module (ADR SQUIRE-A-0010): Keystore-backed session storage, the /pair
// exchange over the generated SDK, NSD host discovery, QR parsing + a ZXing scan screen. Used by
// :app (the single Squire + Knight app) so the pairing flow lives in one place.
android {
    namespace = "com.squire.pairing"
    compileSdk = 34

    defaultConfig {
        minSdk = 26
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlin {
        jvmToolchain(17)
    }

    buildFeatures {
        compose = true
    }
}

dependencies {
    api(project(":sdk"))
    implementation(libs.okhttp)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.kotlinx.coroutines.android)

    // Keystore-backed encrypted token storage (NFR-5).
    implementation(libs.androidx.security.crypto)
    // QR scanning (camera) — ZXing, fully on-device, no Play Services.
    implementation(libs.zxing.android.embedded)

    val composeBom = platform(libs.compose.bom)
    implementation(composeBom)
    implementation(libs.compose.material3)
    implementation(libs.compose.ui)
    implementation(libs.compose.ui.tooling.preview)
    debugImplementation(libs.compose.ui.tooling)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)

    testImplementation(kotlin("test"))
}
