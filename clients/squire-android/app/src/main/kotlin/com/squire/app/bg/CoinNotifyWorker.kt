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
import com.squire.app.MainActivity
import com.squire.app.data.SquireApiAdapter
import com.squire.pairing.SessionStore
import java.util.concurrent.TimeUnit

/**
 * Background coin-balance watcher (SQUIRE-T-0094). WorkManager runs this periodically even while the
 * app is backgrounded; it fetches the Squire's `GET /state` and, if the coin balance has gone **up**
 * since we last saw it, posts a notification so the child knows they earned coins — e.g. when a parent
 * grants coins or approves a quest while the app is closed (the original gap: "he got the coins, just
 * didn't know").
 *
 * The baseline is shared with the foreground app via [recordSeenBalance]: whenever the app shows the
 * balance, it updates the baseline, so the child is never re-notified for coins they already saw
 * in-app. Best-effort — no session or any failure just retries next period.
 */
class CoinNotifyWorker(ctx: Context, params: WorkerParameters) : CoroutineWorker(ctx, params) {

    override suspend fun doWork(): Result {
        val session = SessionStore(applicationContext).load() ?: return Result.success()
        val state = try {
            SquireApiAdapter(session.baseUrl, session.household, session.token).fetchState()
        } catch (_: Exception) {
            return Result.success() // offline / unreachable — try next period
        }

        val prefs = prefs(applicationContext)
        val last = prefs.getInt(KEY_BALANCE, -1)
        val now = state.balance
        // Only notify on a genuine increase past a known baseline (the first run just records it).
        if (last in 0 until now) {
            notifyCoins(applicationContext, gained = now - last, total = now)
        }
        prefs.edit().putInt(KEY_BALANCE, now).apply()
        return Result.success()
    }

    private fun notifyCoins(ctx: Context, gained: Int, total: Int) {
        val mgr = ctx.getSystemService(NotificationManager::class.java) ?: return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            mgr.createNotificationChannel(
                NotificationChannel(CHANNEL, "Coins", NotificationManager.IMPORTANCE_DEFAULT),
            )
        }
        val open = PendingIntent.getActivity(
            ctx,
            0,
            Intent(ctx, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = NotificationCompat.Builder(ctx, CHANNEL)
            .setSmallIcon(android.R.drawable.btn_star_big_on)
            .setContentTitle("You earned coins! 🪙")
            .setContentText("+$gained coins — you now have $total. Tap to see.")
            .setAutoCancel(true)
            .setContentIntent(open)
            .build()
        // POST_NOTIFICATIONS (API 33+): if the grant is missing the post is a silent no-op.
        runCatching { mgr.notify(NOTIF_ID, notification) }
    }

    companion object {
        private const val CHANNEL = "squire_coins"
        private const val NOTIF_ID = 4921
        private const val WORK = "squire-coin-notify"
        private const val PREFS = "coin_notify"
        private const val KEY_BALANCE = "last_balance"

        /** Enqueue the periodic balance watch (idempotent — keeps any existing schedule). */
        fun schedule(ctx: Context) {
            val request = PeriodicWorkRequestBuilder<CoinNotifyWorker>(1, TimeUnit.HOURS)
                .setConstraints(
                    Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build(),
                )
                .build()
            WorkManager.getInstance(ctx)
                .enqueueUniquePeriodicWork(WORK, ExistingPeriodicWorkPolicy.KEEP, request)
        }

        /** Stop the watch (e.g. on "forget device"). */
        fun cancel(ctx: Context) {
            WorkManager.getInstance(ctx).cancelUniqueWork(WORK)
        }

        /**
         * Record the balance the child has just *seen* in the foreground app, so the background watcher
         * uses it as the baseline and never re-notifies for coins already visible on screen.
         */
        fun recordSeenBalance(ctx: Context, balance: Int) {
            prefs(ctx).edit().putInt(KEY_BALANCE, balance).apply()
        }

        private fun prefs(ctx: Context) =
            ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    }
}
