package com.squire.app.bg

import android.content.Context
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.knight.core.QueueNotice
import com.squire.pairing.SessionStore
import com.squire.sdk.model.HouseholdReview
import java.util.concurrent.TimeUnit

/**
 * The Knight's background watcher (SQUIRE-T-0139) — the notification the product was missing.
 *
 * A parent is the bottleneck: a chore sits unsealed until they happen to open the app, and until now
 * nothing told them there was anything to open it for. This polls the household review and says so.
 *
 * Two deliberate choices:
 *  * **No quiet hours.** A Knight is an adult who manages their own phone's do-not-disturb; the
 *    waking window exists to protect a child's evening (SQUIRE-I-0007).
 *  * **Say it once.** Every item is keyed, so a queue left unsealed goes quiet after the first word
 *    rather than nagging every fifteen minutes. A parent who ignored it meant to.
 */
class KnightNotifyWorker(ctx: Context, params: WorkerParameters) : CoroutineWorker(ctx, params) {

    override suspend fun doWork(): Result {
        val session = SessionStore(applicationContext).load() ?: return Result.success()
        val review = try {
            KnightApiAdapter(session.baseUrl, session.household, session.token).fetch()
        } catch (_: Exception) {
            return Result.success() // offline / unreachable — try next period
        }
        announce(applicationContext, review)
        return Result.success()
    }

    companion object {
        private const val WORK = "knight-notify"

        /** Say what is newly waiting, if anything. Shared with the foreground so tests can drive it. */
        internal fun announce(ctx: Context, review: HouseholdReview) {
            val spoken = Notifier.spoken(ctx)
            val notice = QueueNotice.notice(review, spoken) ?: return
            Notifier.post(
                ctx = ctx,
                channel = Notifier.Channel.Seals,
                key = notice.keys.first(),
                title = notice.title,
                text = notice.text,
                tab = TAB_REVIEW,
                respectQuietHours = false,
                // One slot, so a second arrival replaces the first rather than stacking up.
                notificationId = NOTIF_ID,
            )
            // Every item in the notice is now spoken for, not just the key it was posted under.
            notice.keys.forEach { Notifier.remember(ctx, it) }
        }

        /** Enqueue the periodic queue watch (idempotent). */
        fun schedule(ctx: Context) {
            val request = PeriodicWorkRequestBuilder<KnightNotifyWorker>(15, TimeUnit.MINUTES)
                .setConstraints(
                    Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build(),
                )
                .build()
            WorkManager.getInstance(ctx)
                .enqueueUniquePeriodicWork(WORK, ExistingPeriodicWorkPolicy.UPDATE, request)
        }

        /** Stop the watch (e.g. on "forget device"). */
        fun cancel(ctx: Context) {
            WorkManager.getInstance(ctx).cancelUniqueWork(WORK)
        }

        private const val NOTIF_ID = 4922
        internal const val TAB_REVIEW = "Review"
    }
}
