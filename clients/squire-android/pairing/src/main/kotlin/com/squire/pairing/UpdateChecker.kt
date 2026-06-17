package com.squire.pairing

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.OkHttpClient
import okhttp3.Request

/** One app's entry in the server's update manifest (`GET /app/manifest`, SQUIRE-T-0051). */
@Serializable
data class AppRelease(val versionCode: Int, val versionName: String, val file: String)

/** The server's update manifest — a release per app (absent = no build published for it). */
@Serializable
data class UpdateManifest(val squire: AppRelease? = null, val knight: AppRelease? = null)

/** A newer build the server is offering for this app, with the LAN download URL. */
data class UpdateInfo(val versionCode: Int, val versionName: String, val downloadUrl: String)

/**
 * Checks the LAN server's `/app/manifest` for a newer build of this app (ADR-less LAN sideload,
 * SQUIRE-T-0051). The phone can't silently install a sideloaded APK, so the UI just surfaces an
 * "Update available" banner whose action opens the download URL in the browser.
 */
object UpdateChecker {
    private val json = Json { ignoreUnknownKeys = true }
    private val client = OkHttpClient()

    /**
     * Returns an [UpdateInfo] when the server advertises a higher `versionCode` for [appKey]
     * (`"squire"` or `"knight"`) than [currentVersionCode]; otherwise null. Never throws — any
     * network/parse failure (e.g. the server has no `SQUIRE_APK_DIR`) is simply "no update".
     */
    suspend fun check(baseUrl: String, appKey: String, currentVersionCode: Int): UpdateInfo? =
        withContext(Dispatchers.IO) {
            runCatching {
                val req = Request.Builder().url("$baseUrl/app/manifest").build()
                client.newCall(req).execute().use { resp ->
                    val body = resp.body?.string()
                    if (!resp.isSuccessful || body == null) return@use null
                    val manifest = json.decodeFromString(UpdateManifest.serializer(), body)
                    val rel = if (appKey == "knight") manifest.knight else manifest.squire
                    if (rel != null && rel.versionCode > currentVersionCode) {
                        UpdateInfo(rel.versionCode, rel.versionName, "$baseUrl/app/${rel.file}")
                    } else {
                        null
                    }
                }
            }.getOrNull()
        }
}

/** A thin "Update available" strip; the action ([onGetUpdate]) opens the APK URL in the browser. */
@Composable
fun UpdateBanner(info: UpdateInfo, onGetUpdate: () -> Unit) {
    Surface(color = MaterialTheme.colorScheme.tertiaryContainer, modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(
                "Update available — v${info.versionName}",
                color = MaterialTheme.colorScheme.onTertiaryContainer,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.weight(1f),
            )
            TextButton(onClick = onGetUpdate) { Text("Get update") }
        }
    }
}
