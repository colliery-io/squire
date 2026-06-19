package com.squire.app.bg

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.squire.app.BuildConfig
import com.squire.app.MainActivity
import com.squire.pairing.SessionStore
import com.squire.pairing.UpdateChecker
import java.util.concurrent.TimeUnit

/**
 * Background app-update check (SQUIRE-T-0090 / ADR SQUIRE-A-0012). WorkManager runs this periodically
 * even while the app is backgrounded; it asks the paired home server whether a newer APK is published
 * (content-hash detection, SQUIRE-T-0085) and, if so, posts a notification the user taps to open the
 * app and install. Battery-friendly + OS-scheduled — no always-on foreground service. Best-effort:
 * no session or any failure just retries on the next period.
 */
class UpdateCheckWorker(ctx: Context, params: WorkerParameters) : CoroutineWorker(ctx, params) {

    override suspend fun doWork(): Result {
        val session = SessionStore(applicationContext).load() ?: return Result.success()
        val info = UpdateChecker.check(
            applicationContext,
            session.baseUrl,
            "squire",
            BuildConfig.VERSION_CODE,
        ) ?: return Result.success() // up to date / unreachable

        notifyUpdateAvailable(applicationContext, info.versionName)
        return Result.success()
    }

    private fun notifyUpdateAvailable(ctx: Context, versionName: String) {
        val mgr = ctx.getSystemService(NotificationManager::class.java) ?: return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            mgr.createNotificationChannel(
                NotificationChannel(CHANNEL, "App updates", NotificationManager.IMPORTANCE_DEFAULT),
            )
        }
        val open = PendingIntent.getActivity(
            ctx,
            0,
            Intent(ctx, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = NotificationCompat.Builder(ctx, CHANNEL)
            .setSmallIcon(android.R.drawable.stat_sys_download_done)
            .setContentTitle("Squire update available")
            .setContentText("Tap to install v$versionName over your home Wi-Fi.")
            .setAutoCancel(true)
            .setContentIntent(open)
            .build()
        // POST_NOTIFICATIONS (API 33+): if the grant is missing the post is a silent no-op.
        runCatching { mgr.notify(NOTIF_ID, notification) }
    }

    companion object {
        private const val CHANNEL = "squire_updates"
        private const val NOTIF_ID = 4920
        private const val WORK = "squire-update-check"

        /** Enqueue the periodic background check (idempotent — keeps any existing schedule). */
        fun schedule(ctx: Context) {
            val request = PeriodicWorkRequestBuilder<UpdateCheckWorker>(6, TimeUnit.HOURS)
                .setConstraints(
                    Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build(),
                )
                .build()
            WorkManager.getInstance(ctx)
                .enqueueUniquePeriodicWork(WORK, ExistingPeriodicWorkPolicy.KEEP, request)
        }

        /** Stop the periodic check (e.g. on "forget device"). */
        fun cancel(ctx: Context) {
            WorkManager.getInstance(ctx).cancelUniqueWork(WORK)
        }
    }
}
