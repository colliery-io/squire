package com.squire.pairing

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
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
 * SQUIRE-T-0051). When one is offered, [UpdateBanner] downloads and installs it **in-app** via
 * [AppUpdater] (SQUIRE-T-0059) — the system install prompt, no browser.
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

/** Banner state for the in-app update (SQUIRE-T-0059). */
private sealed interface BannerState {
    data object Idle : BannerState
    data class Downloading(val percent: Int) : BannerState
    data class Failed(val message: String) : BannerState
}

/**
 * An "Update available" strip that installs the new build **in-app** (SQUIRE-T-0059): tap "Get
 * update" → the APK downloads here (progress shown) → the system install prompt appears. No browser.
 * Self-contained — the host just renders it with the [info] from [UpdateChecker.check].
 */
@Composable
fun UpdateBanner(info: UpdateInfo) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var state by remember(info.versionCode) { mutableStateOf<BannerState>(BannerState.Idle) }

    Surface(color = MaterialTheme.colorScheme.tertiaryContainer, modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    "Update available — v${info.versionName}",
                    color = MaterialTheme.colorScheme.onTertiaryContainer,
                    fontWeight = FontWeight.Medium,
                )
                when (val s = state) {
                    is BannerState.Downloading -> Text(
                        if (s.percent >= 0) "Downloading… ${s.percent}%" else "Downloading…",
                        color = MaterialTheme.colorScheme.onTertiaryContainer,
                        style = MaterialTheme.typography.bodySmall,
                    )
                    is BannerState.Failed -> Text(
                        s.message,
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall,
                    )
                    BannerState.Idle -> Unit
                }
            }

            if (state is BannerState.Downloading) {
                CircularProgressIndicator(
                    strokeWidth = 2.dp,
                    color = MaterialTheme.colorScheme.onTertiaryContainer,
                    modifier = Modifier.size(20.dp),
                )
            } else {
                TextButton(onClick = {
                    state = BannerState.Downloading(0)
                    scope.launch {
                        val err = AppUpdater.downloadAndInstall(context, info.downloadUrl) { pct ->
                            state = BannerState.Downloading(pct)
                        }
                        state = if (err == null) BannerState.Idle else BannerState.Failed(err)
                    }
                }) {
                    Text(if (state is BannerState.Failed) "Retry" else "Get update")
                }
            }
        }
    }
}
