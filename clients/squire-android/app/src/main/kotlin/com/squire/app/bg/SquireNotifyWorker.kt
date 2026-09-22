package com.squire.app.bg

import android.content.Context
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.squire.app.data.SquireApiAdapter
import com.squire.pairing.SessionStore
import com.squire.core.NotifyPolicy
import com.squire.sdk.model.StateView
import java.util.concurrent.TimeUnit

/**
 * The squire's background watcher (SQUIRE-T-0138, formerly the coin-only `CoinNotifyWorker` of
 * SQUIRE-T-0094). It polls `GET /state` and tells the child what a grown-up decided:
 *
 *  * **"Dad sealed Tidy your room · +10 coins"** — a claim reached approval.
 *  * **"Dad said not yet — <reason>"** — a claim was declined. This was previously silent: the child
 *    only ever discovered it by opening the app, which is a poor way to hear you have to redo
 *    something.
 *  * **"You earned coins"** — only for a rise the seals do *not* account for (a direct grant), so the
 *    same coins are never announced twice.
 *
 * Every decision here is made by [NotifyPolicy] and tested on the JVM; this class is the wiring.
 * Per ADR SQUIRE-A-0017 nothing is pushed — the phone asks, on WorkManager's cadence.
 */
class SquireNotifyWorker(ctx: Context, params: WorkerParameters) : CoroutineWorker(ctx, params) {

    override suspend fun doWork(): Result {
        val session = SessionStore(applicationContext).load() ?: return Result.success()
        val state = try {
            SquireApiAdapter(session.baseUrl, session.household, session.token).fetchState()
        } catch (_: Exception) {
            return Result.success() // offline / unreachable — try next period
        }
        announce(applicationContext, state)
        return Result.success()
    }

    companion object {
        private const val WORK = "squire-notify"
        private const val PREFS = "coin_notify" // kept: an installed app already has its baseline here
        private const val KEY_BALANCE = "last_balance"
        private const val KEY_CLAIMS = "last_claims"

        /** Announce anything new in [state], then record what was seen. Shared with the foreground. */
        internal fun announce(ctx: Context, state: StateView) {
            val prefs = prefs(ctx)
            val claims = state.myClaims.orEmpty()
            // A first run has nothing to compare against, so it records and stays quiet.
            val previous = prefs.getString(KEY_CLAIMS, null)
                ?.split("\n")?.filter { it.isNotEmpty() }?.toSet()

            val outcomes = NotifyPolicy.outcomes(previous, claims)
            val settings = state.notify

            // Several verdicts at once (a parent clearing the queue) collapse into one line rather
            // than buzzing a child's phone four times.
            if (outcomes.size > COLLAPSE_ABOVE) {
                val sealed = outcomes.count { it.sealed }
                val coins = outcomes.filter { it.sealed }.sumOf { it.coins }
                Notifier.post(
                    ctx = ctx,
                    channel = Notifier.Channel.Seals,
                    key = "batch:" + outcomes.joinToString(",") { it.key },
                    title = if (sealed == outcomes.size) "$sealed chores sealed! 🪙" else "A grown-up reviewed your chores",
                    text = buildString {
                        if (sealed > 0) append("$sealed sealed")
                        if (coins > 0) append(" · +$coins coins")
                        val notYet = outcomes.size - sealed
                        if (notYet > 0) append("${if (sealed > 0) " · " else ""}$notYet to try again")
                    }.trim(),
                    tab = TAB_TODAY,
                    settings = settings,
                )
            } else {
                for (o in outcomes) {
                    if (o.sealed) {
                        Notifier.post(
                            ctx = ctx,
                            channel = Notifier.Channel.Seals,
                            key = o.key,
                            title = "Sealed! 🪙",
                            text = if (o.coins > 0) "${o.questTitle} · +${o.coins} coins" else o.questTitle,
                            tab = TAB_TODAY,
                            settings = settings,
                        )
                    } else {
                        Notifier.post(
                            ctx = ctx,
                            channel = Notifier.Channel.Seals,
                            key = o.key,
                            title = "Not yet — ${o.questTitle}",
                            text = o.reason ?: "Have another go and tap “I did it” again.",
                            tab = TAB_TODAY,
                            settings = settings,
                        )
                    }
                }
            }

            // Coins that no seal explains — a grown-up granting them directly.
            val last = prefs.getInt(KEY_BALANCE, -1)
            val gained = NotifyPolicy.unexplainedCoinGain(last, state.balance, outcomes)
            if (gained > 0) {
                Notifier.post(
                    ctx = ctx,
                    channel = Notifier.Channel.Seals,
                    key = "coins:${state.generatedAt}",
                    title = "You earned coins! 🪙",
                    text = "+$gained coins — you now have ${state.balance}.",
                    tab = TAB_TODAY,
                    settings = settings,
                )
            }

            prefs.edit()
                .putInt(KEY_BALANCE, state.balance)
                .putString(KEY_CLAIMS, NotifyPolicy.markers(claims).joinToString("\n"))
                .apply()

            // Keep the chore-time reminders armed from the settings we just saw, so a change a
            // parent makes in the Keep lands on the phone within a poll (SQUIRE-T-0140).
            ChoreReminderWorker.schedule(ctx, state.notify?.choreTimes.orEmpty(), state.notify?.timezone)
        }

        /**
         * Record what the child is looking at right now, without announcing any of it — the
         * foreground courtesy that keeps the watcher from re-telling them what is already on screen
         * (SQUIRE-T-0094's `recordSeenBalance`, widened to verdicts).
         */
        fun recordSeen(ctx: Context, state: StateView) {
            prefs(ctx).edit()
                .putInt(KEY_BALANCE, state.balance)
                .putString(KEY_CLAIMS, NotifyPolicy.markers(state.myClaims.orEmpty()).joinToString("\n"))
                .apply()
        }

        /** Enqueue the periodic watch (idempotent — keeps any existing schedule). */
        fun schedule(ctx: Context) {
            val request = PeriodicWorkRequestBuilder<SquireNotifyWorker>(15, TimeUnit.MINUTES)
                .setConstraints(
                    Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build(),
                )
                .build()
            WorkManager.getInstance(ctx)
                // REPLACE, not KEEP: an install carrying the old hourly schedule must pick up the
                // 15-minute one rather than keep its own forever.
                .enqueueUniquePeriodicWork(WORK, ExistingPeriodicWorkPolicy.UPDATE, request)
        }

        /** Stop the watch (e.g. on "forget device") and forget what we have said. */
        fun cancel(ctx: Context) {
            WorkManager.getInstance(ctx).cancelUniqueWork(WORK)
            WorkManager.getInstance(ctx).cancelUniqueWork("squire-coin-notify") // the pre-T-0138 name
            prefs(ctx).edit().clear().apply()
            Notifier.clear(ctx)
        }

        /** Above this many verdicts in one poll, say one thing instead of many. */
        private const val COLLAPSE_ABOVE = 2
        internal const val TAB_TODAY = "Quests"

        private fun prefs(ctx: Context) = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    }
}
