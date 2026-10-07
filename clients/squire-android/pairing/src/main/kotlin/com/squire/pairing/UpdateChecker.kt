package com.squire.pairing

import android.content.Context
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

/**
 * One app's entry in the server's update manifest (`GET /app/manifest`, SQUIRE-T-0051). [sha256] is
 * the server-computed content hash of the APK (SQUIRE-T-0085) — present once the server is on the
 * hash-aware build, absent against older servers.
 */
@Serializable
data class AppRelease(
    val versionCode: Int,
    val versionName: String,
    val file: String,
    val sha256: String? = null,
)

/** The server's update manifest — a release per app (absent = no build published for it). */
@Serializable
data class UpdateManifest(val squire: AppRelease? = null, val knight: AppRelease? = null)

/** A build the server is offering for this app, with the LAN download URL and its content hash. */
data class UpdateInfo(
    val appKey: String,
    val versionCode: Int,
    val versionName: String,
    val downloadUrl: String,
    val sha256: String?,
)

/**
 * Checks the LAN server's `/app/manifest` for an update of this app (ADR-less LAN sideload,
 * SQUIRE-T-0051). When one is offered, [UpdateBanner] downloads and installs it **in-app** via
 * [AppUpdater] (SQUIRE-T-0059) — the system install prompt, no browser.
 *
 * Detection is **content-hash based** (SQUIRE-T-0085, phase 2): an update is offered when the
 * server's `sha256` differs from the SHA-256 of this app's own installed APK. A single-APK install
 * keeps the exact bytes as `base.apk`, so the hash names what is really installed. Phase 1 kept a
 * stored "last installed" hash instead, which went wrong when the user cancelled the install prompt.
 * `versionCode` is only a downgrade guard now (see [UpdateDecision.shouldOffer]).
 */
object UpdateChecker {
    private val json = Json { ignoreUnknownKeys = true }
    private val client = OkHttpClient()

    private const val PREFS = "squire_ota"
    private const val KEY_HASHED_APK = "self_hash_of"
    private const val KEY_HASH = "self_sha256"

    /**
     * SHA-256 of this app's installed APK, or null when it can't be read. Cached against the APK
     * path and install time, so the APK is hashed once per install, not on every poll.
     */
    private fun installedApkHash(context: Context): String? = runCatching {
        val app = context.applicationContext
        val apk = app.applicationInfo.sourceDir
        val installedAt = app.packageManager.getPackageInfo(app.packageName, 0).lastUpdateTime
        val identity = "$apk@$installedAt"
        val prefs = app.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        prefs.getString(KEY_HASH, null)?.takeIf { prefs.getString(KEY_HASHED_APK, null) == identity }
            ?: UpdateDecision.sha256Hex(java.io.File(apk)).also { hash ->
                prefs.edit().putString(KEY_HASHED_APK, identity).putString(KEY_HASH, hash).apply()
            }
    }.getOrNull()

    /**
     * Returns an [UpdateInfo] when the server offers a build of [appKey] (`"squire"` or `"knight"`)
     * whose content differs from what we have. Never throws — any network/parse failure (e.g. the
     * server has no `SQUIRE_APK_DIR`) is simply "no update".
     */
    suspend fun check(
        context: Context,
        baseUrl: String,
        appKey: String,
        currentVersionCode: Int,
    ): UpdateInfo? =
        withContext(Dispatchers.IO) {
            runCatching {
                val req = Request.Builder().url("$baseUrl/app/manifest").build()
                client.newCall(req).execute().use { resp ->
                    val body = resp.body?.string()
                    if (!resp.isSuccessful || body == null) return@use null
                    val manifest = json.decodeFromString(UpdateManifest.serializer(), body)
                    val rel = (if (appKey == "knight") manifest.knight else manifest.squire)
                        ?: return@use null

                    val offer = UpdateDecision.shouldOffer(
                        serverHash = rel.sha256,
                        installedHash = installedApkHash(context),
                        serverVersionCode = rel.versionCode,
                        currentVersionCode = currentVersionCode,
                    )
                    if (!offer) return@use null
                    UpdateInfo(appKey, rel.versionCode, rel.versionName, "$baseUrl/app/${rel.file}", rel.sha256)
                }
            }.getOrNull()
        }
}

/** Banner state for the in-app update (SQUIRE-T-0059). */
private sealed interface BannerState {
    data object Idle : BannerState
    data class Downloading(val percent: Int) : BannerState
    data object Installing : BannerState
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
    var state by remember(info.sha256 ?: info.versionCode) { mutableStateOf<BannerState>(BannerState.Idle) }

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
                        if (s.percent >= 0) "Downloading update… ${s.percent}%" else "Downloading update…",
                        color = MaterialTheme.colorScheme.onTertiaryContainer,
                        style = MaterialTheme.typography.bodySmall,
                    )
                    BannerState.Installing -> Text(
                        "Downloaded — tap “Install” in the system prompt to finish.",
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
                        state = if (err == null) BannerState.Installing else BannerState.Failed(err)
                    }
                }) {
                    Text(if (state is BannerState.Failed) "Retry" else "Update now")
                }
            }
        }
    }
}
