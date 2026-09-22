package com.squire.app.bg

import android.content.Context
import androidx.work.CoroutineWorker
import androidx.work.ExistingWorkPolicy
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.squire.app.data.RoomStateCache
import com.squire.app.data.SquireApiAdapter
import com.squire.app.data.db.SquireDb
import com.squire.core.NotifyPolicy
import com.squire.pairing.SessionStore
import com.squire.sdk.model.StateView
import kotlinx.serialization.json.Json
import java.time.Duration
import java.time.LocalDate
import java.time.ZoneId
import java.time.ZonedDateTime
import java.util.concurrent.TimeUnit

/**
 * The chore-time nudge (SQUIRE-T-0140): at the times a parent set in the Keep, tell the squire what
 * is still to do — **and say nothing at all when they are done**.
 *
 * Scheduled by the phone itself, not pushed (ADR SQUIRE-A-0017), which is what lets it work with the
 * server unreachable: if the fetch fails it falls back to the Room-cached state, which is the same
 * thing the child's screen would be showing.
 *
 * Scheduling uses a one-shot with an initial delay and re-arms on each run, rather than an exact
 * alarm — "around 4:30" is the requirement, and it keeps the app clear of `SCHEDULE_EXACT_ALARM`
 * and its Play-policy baggage.
 */
class ChoreReminderWorker(ctx: Context, params: WorkerParameters) : CoroutineWorker(ctx, params) {

    override suspend fun doWork(): Result {
        val ctx = applicationContext
        val slot = inputData.getInt(KEY_SLOT, 0)
        val session = SessionStore(ctx).load() ?: return Result.success()

        // Fresh if we can reach the server, otherwise the cache the child's own screen would show.
        val state = try {
            SquireApiAdapter(session.baseUrl, session.household, session.token).fetchState()
        } catch (_: Exception) {
            cached(ctx)
        }
        if (state != null) announce(ctx, state, slot)

        // Re-arm for tomorrow from the settings we just saw (a parent may have moved the times).
        schedule(ctx, state?.notify?.choreTimes.orEmpty(), state?.notify?.timezone)
        return Result.success()
    }

    private fun cached(ctx: Context): StateView? = runCatching {
        val raw = RoomStateCache(SquireDb.build(ctx).cacheDao()).load() ?: return null
        Json { ignoreUnknownKeys = true }.decodeFromString(StateView.serializer(), raw)
    }.getOrNull()

    companion object {
        private const val WORK_PREFIX = "squire-chore-reminder-"
        private const val KEY_SLOT = "slot"

        /** Post the nudge for [slot], unless there is nothing left (the rule that keeps it worth reading). */
        internal fun announce(ctx: Context, state: StateView, slot: Int) {
            val (title, text) = NotifyPolicy.choreReminder(state.questsToday) ?: return
            val zone = zoneOf(state.notify?.timezone)
            Notifier.post(
                ctx = ctx,
                channel = Notifier.Channel.Reminders,
                // One per slot per day: a re-arm or a reboot cannot say the same thing twice.
                key = "reminder:${LocalDate.now(zone)}:$slot",
                title = title,
                text = text,
                tab = SquireNotifyWorker.TAB_TODAY,
                settings = state.notify,
                notificationId = NOTIF_BASE + slot,
            )
        }

        /**
         * Arm one one-shot per chore time, each for its next occurrence in the household's zone.
         * Called after every state fetch, so a change made in the Keep lands within a poll.
         */
        fun schedule(ctx: Context, choreTimes: List<Int>, timezone: String?) {
            val wm = WorkManager.getInstance(ctx)
            val zone = zoneOf(timezone)
            // Cancel every slot first: a parent dropping from two times to one must not leave the
            // second armed forever.
            for (slot in 0 until MAX_SLOTS) wm.cancelUniqueWork(WORK_PREFIX + slot)

            choreTimes.take(MAX_SLOTS).forEachIndexed { slot, minutes ->
                val delay = untilNext(minutes, zone)
                val request = OneTimeWorkRequestBuilder<ChoreReminderWorker>()
                    .setInitialDelay(delay.toMinutes().coerceAtLeast(1), TimeUnit.MINUTES)
                    .setInputData(androidx.work.Data.Builder().putInt(KEY_SLOT, slot).build())
                    .build()
                wm.enqueueUniqueWork(WORK_PREFIX + slot, ExistingWorkPolicy.REPLACE, request)
            }
        }

        /** Stop every reminder (unpair, or a Knight device). */
        fun cancel(ctx: Context) {
            val wm = WorkManager.getInstance(ctx)
            for (slot in 0 until MAX_SLOTS) wm.cancelUniqueWork(WORK_PREFIX + slot)
        }

        /** How long until [minuteOfDay] next comes round in [zone] — tomorrow if it has passed today. */
        internal fun untilNext(minuteOfDay: Int, zone: ZoneId, from: ZonedDateTime? = null): Duration {
            val now = from ?: ZonedDateTime.now(zone)
            var next = now.toLocalDate().atStartOfDay(zone).plusMinutes(minuteOfDay.toLong())
            if (!next.isAfter(now)) next = next.plusDays(1)
            return Duration.between(now, next)
        }

        private fun zoneOf(timezone: String?): ZoneId =
            timezone?.let { runCatching { ZoneId.of(it) }.getOrNull() } ?: ZoneId.systemDefault()

        /** Matches `domain_core::contract::MAX_CHORE_TIMES`. */
        private const val MAX_SLOTS = 2
        private const val NOTIF_BASE = 4930
    }
}
