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
 * Detection is **content-hash based** (SQUIRE-T-0085): an update is offered when the server's
 * `sha256` differs from the hash we last installed (persisted locally — the phone trusts the
 * server's advertised hash as the identity of what it installed, it does not re-hash itself). This
 * is phase 1 of the migration, so `versionCode` is retained as the bootstrap-baseline signal and as
 * the fallback against pre-hash servers; a later release drops it for pure hash.
 */
object UpdateChecker {
    private val json = Json { ignoreUnknownKeys = true }
    private val client = OkHttpClient()

    private const val PREFS = "squire_ota"
    private fun hashKey(appKey: String) = "installed_sha256_$appKey"

    /** The content hash we last installed for [appKey], or null if we've never recorded one. */
    private fun installedHash(context: Context, appKey: String): String? =
        context.applicationContext
            .getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .getString(hashKey(appKey), null)

    /**
     * Record [sha256] as the build we now have for [appKey]. Called when an install is committed so
     * the just-installed build doesn't immediately re-advertise itself as an update.
     */
    fun markInstalled(context: Context, appKey: String, sha256: String) {
        context.applicationContext
            .getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .putString(hashKey(appKey), sha256)
            .apply()
    }

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

                    fun offer() = UpdateInfo(
                        appKey, rel.versionCode, rel.versionName, "$baseUrl/app/${rel.file}", rel.sha256,
                    )

                    val serverHash = rel.sha256
                    val stored = installedHash(context, appKey)
                    when {
                        // Hash-aware server.
                        serverHash != null -> when {
                            stored != null -> if (serverHash != stored) offer() else null
                            // No baseline yet: a higher versionCode means we're genuinely the older
                            // build (legacy version-delivered) — offer it. Otherwise the server's
                            // current build IS us, so seed the baseline silently (no prompt).
                            rel.versionCode > currentVersionCode -> offer()
                            else -> { markInstalled(context, appKey, serverHash); null }
                        }
                        // Pre-hash server: fall back to versionCode comparison.
                        rel.versionCode > currentVersionCode -> offer()
                        else -> null
                    }
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
                        if (err == null) {
                            // The exact advertised bytes are staged + committed; record this as the
                            // build we now have so it doesn't re-advertise itself (SQUIRE-T-0085).
                            info.sha256?.let { UpdateChecker.markInstalled(context, info.appKey, it) }
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
