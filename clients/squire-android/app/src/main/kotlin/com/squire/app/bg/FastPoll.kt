package com.squire.app.bg

import android.content.Context
import androidx.work.Constraints
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequest
import androidx.work.WorkManager
import java.util.concurrent.TimeUnit

/**
 * A faster heartbeat than WorkManager's periodic floor (SQUIRE-T-0144).
 *
 * `PeriodicWorkRequest` will not run more often than every 15 minutes — that is a hard platform
 * limit, not a setting. A one-shot that re-enqueues itself has no such floor, so this chains a
 * check every [INTERVAL_MINUTES] instead.
 *
 * **What this does and does not buy.** While the phone is awake and the app is in an active standby
 * bucket, a seal or a turned-in chore now surfaces in a minute or two rather than up to fifteen.
 * Once the phone is in Doze — screen off, stationary, pocket or bedside — Android batches deferrable
 * work into maintenance windows regardless of what we ask for, and the delay stretches again. No
 * amount of WorkManager scheduling beats Doze; only a high-priority push or a foreground service
 * does, and both were weighed in ADR SQUIRE-A-0017.
 *
 * The chain is fragile by nature — a force-stop or a crash ends it — so the 15-minute periodic
 * worker stays as a backstop and re-arms this on every run. Belt and braces: the fast path for
 * responsiveness, the slow one so silence is never permanent.
 */
object FastPoll {

    /** Often enough to feel prompt, seldom enough that a small JSON fetch costs nothing much. */
    const val INTERVAL_MINUTES = 2L

    private const val SQUIRE_WORK = "squire-notify-fast"
    private const val KNIGHT_WORK = "knight-notify-fast"

    /** Arm (or re-arm) the fast chain for whichever role this device is paired as. */
    fun arm(ctx: Context, knight: Boolean) {
        if (knight) {
            enqueue(ctx, KNIGHT_WORK, OneTimeWorkRequest.Builder(KnightNotifyWorker::class.java))
            WorkManager.getInstance(ctx).cancelUniqueWork(SQUIRE_WORK)
        } else {
            enqueue(ctx, SQUIRE_WORK, OneTimeWorkRequest.Builder(SquireNotifyWorker::class.java))
            WorkManager.getInstance(ctx).cancelUniqueWork(KNIGHT_WORK)
        }
    }

    /** Stop both chains (unpair). */
    fun cancel(ctx: Context) {
        val wm = WorkManager.getInstance(ctx)
        wm.cancelUniqueWork(SQUIRE_WORK)
        wm.cancelUniqueWork(KNIGHT_WORK)
    }

    private fun enqueue(ctx: Context, name: String, builder: OneTimeWorkRequest.Builder) {
        val request = builder
            .setInitialDelay(INTERVAL_MINUTES, TimeUnit.MINUTES)
            .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .addTag(TAG)
            .build()
        // KEEP, not REPLACE: a chain already ticking must not be reset to a fresh two minutes every
        // time something re-arms it, or a frequent re-arm would starve it of ever running.
        WorkManager.getInstance(ctx).enqueueUniqueWork(name, ExistingWorkPolicy.KEEP, request)
    }

    /**
     * Re-arm from inside a worker that has just finished, continuing the chain.
     *
     * **The cancel is load-bearing — do not "simplify" it away.** This runs from inside the worker,
     * so that worker's own unique-work record is still RUNNING, and `KEEP` would see unfinished work
     * under the name and silently drop the next tick. The chain would then stop dead, with nothing
     * failing and nothing logged; only the 15-minute backstop would ever revive it.
     *
     * Cancelling first means the worker cancels itself, which is fine: it is one statement from
     * returning, and a cancelled worker's `Result` is discarded anyway. The new request is enqueued
     * afterwards, so it is not caught by the cancel.
     */
    fun rearm(ctx: Context, knight: Boolean) {
        val wm = WorkManager.getInstance(ctx)
        wm.cancelUniqueWork(if (knight) KNIGHT_WORK else SQUIRE_WORK)
        arm(ctx, knight)
    }

    const val TAG = "squire-fast-poll"
}
