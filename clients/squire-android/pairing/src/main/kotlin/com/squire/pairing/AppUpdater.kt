package com.squire.pairing

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.os.Build
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request

/**
 * In-app updater (SQUIRE-T-0059): downloads the LAN APK and installs it through Android's
 * [PackageInstaller] session API — no browser, no Downloads app. The phone can't silently install a
 * sideloaded APK (we're not a device owner), so the **system** "update Squire?" dialog is still
 * shown; we just get the user straight there. First use prompts a one-time "install unknown apps"
 * grant for Squire itself.
 *
 * The committed session reports back through a [PendingIntent] broadcast to [InstallReceiver], which
 * launches the system confirmation when the session is [PackageInstaller.STATUS_PENDING_USER_ACTION].
 */
object AppUpdater {
    private val client = OkHttpClient()

    /** Progress while streaming the APK into the install session: 0..100, or -1 when indeterminate. */
    fun interface Progress {
        fun onPercent(percent: Int)
    }

    /**
     * Download [downloadUrl] and stage+commit it into a [PackageInstaller] session. Returns null on
     * success (the system installer takes over); a human-readable message on failure. Reports
     * download progress via [progress]. Runs IO off the main thread.
     */
    suspend fun downloadAndInstall(
        context: Context,
        downloadUrl: String,
        progress: Progress,
    ): String? = withContext(Dispatchers.IO) {
        runCatching {
            val appContext = context.applicationContext
            val installer = appContext.packageManager.packageInstaller
            val params = PackageInstaller.SessionParams(
                PackageInstaller.SessionParams.MODE_FULL_INSTALL,
            )
            params.setAppPackageName(appContext.packageName)

            val req = Request.Builder().url(downloadUrl).build()
            client.newCall(req).execute().use { resp ->
                val body = resp.body
                if (!resp.isSuccessful || body == null) {
                    return@withContext "Download failed (HTTP ${resp.code}). Is the computer reachable?"
                }
                val total = body.contentLength()
                if (total > 0) params.setSize(total)

                val sessionId = installer.createSession(params)
                installer.openSession(sessionId).use { session ->
                    session.openWrite("squire-update", 0, total).use { out ->
                        body.byteStream().use { input ->
                            val buf = ByteArray(64 * 1024)
                            var written = 0L
                            var lastPct = -1
                            while (true) {
                                val n = input.read(buf)
                                if (n < 0) break
                                out.write(buf, 0, n)
                                written += n
                                if (total > 0) {
                                    val pct = ((written * 100) / total).toInt()
                                    if (pct != lastPct) {
                                        lastPct = pct
                                        progress.onPercent(pct)
                                    }
                                } else {
                                    progress.onPercent(-1)
                                }
                            }
                            session.fsync(out)
                        }
                    }

                    val statusIntent = Intent(appContext, InstallReceiver::class.java)
                        .setAction(InstallReceiver.ACTION_INSTALL_STATUS)
                    val flags = PendingIntent.FLAG_UPDATE_CURRENT or
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0
                    val pending = PendingIntent.getBroadcast(appContext, sessionId, statusIntent, flags)
                    session.commit(pending.intentSender)
                }
            }
            null
        }.getOrElse { e ->
            "Update failed: ${e.message ?: e.javaClass.simpleName}"
        }
    }
}

/**
 * Receives [PackageInstaller] session callbacks for the in-app update (SQUIRE-T-0059). When the
 * session needs the user to confirm ([PackageInstaller.STATUS_PENDING_USER_ACTION]) it launches the
 * system install dialog; success/failure are otherwise terminal (the app is replaced on success).
 * Declared in the app manifest, not exported.
 */
class InstallReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != ACTION_INSTALL_STATUS) return
        when (intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)) {
            PackageInstaller.STATUS_PENDING_USER_ACTION -> {
                @Suppress("DEPRECATION")
                val confirm = intent.getParcelableExtra<Intent>(Intent.EXTRA_INTENT)
                confirm?.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                confirm?.let { context.startActivity(it) }
            }
            // SUCCESS / FAILURE* are terminal; on success the system replaces the app. Nothing to do.
            else -> Unit
        }
    }

    companion object {
        const val ACTION_INSTALL_STATUS = "com.squire.pairing.INSTALL_STATUS"
    }
}
