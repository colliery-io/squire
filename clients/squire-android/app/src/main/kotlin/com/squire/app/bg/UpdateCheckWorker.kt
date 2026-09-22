package com.squire.app.bg

import android.content.Context
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.squire.app.BuildConfig
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
        // Through the one Notifier (SQUIRE-T-0138) so this shares the channel set, the
        // POST_NOTIFICATIONS handling and the never-twice ledger. Keyed by version, so a build the
        // household has already been told about is not re-announced every six hours — which the
        // ad-hoc version did, because it re-posted the same notification id each run.
        Notifier.post(
            ctx = ctx,
            channel = Notifier.Channel.Updates,
            key = "update:$versionName",
            title = "Squire update available",
            text = "Tap to install v$versionName over your home Wi-Fi.",
            // An app update is worth hearing whenever it lands; it is not a child's nag.
            respectQuietHours = false,
        )
    }

    companion object {
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
