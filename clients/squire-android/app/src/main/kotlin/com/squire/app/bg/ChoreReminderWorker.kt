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

        // Re-arm for tomorrow from what we just saw (a parent may have moved the times, or given a
        // chore a new one).
        schedule(
            ctx,
            state?.notify?.choreTimes.orEmpty(),
            state?.notify?.timezone,
            state?.questsToday?.mapNotNull { it.dueTime }.orEmpty(),
        )
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
            val zone = zoneOf(state.notify?.timezone)
            val today = LocalDate.now(zone)
            if (slot == SLOT_STREAK) {
                announceStreak(ctx, state, today)
                return
            }
            if (slot >= SLOT_DUE_BASE) {
                announceDue(ctx, state, today, ZonedDateTime.now(zone))
                return
            }
            val (title, text) = NotifyPolicy.choreReminder(state.questsToday) ?: return
            Notifier.post(
                ctx = ctx,
                channel = Notifier.Channel.Reminders,
                // One per slot per day: a re-arm or a reboot cannot say the same thing twice.
                key = "reminder:$today:$slot",
                title = title,
                text = text,
                tab = SquireNotifyWorker.TAB_TODAY,
                settings = state.notify,
                notificationId = NOTIF_BASE + slot,
            )
        }

        /**
         * The per-chore nudge (SQUIRE-T-0142): everything falling due around now that is still
         * undone, in **one** notification — three chores at 07:30 must not mean three buzzes.
         */
        private fun announceDue(ctx: Context, state: StateView, today: LocalDate, now: ZonedDateTime) {
            val minute = now.hour * 60 + now.minute
            val due = NotifyPolicy.dueNow(state.questsToday, minute)
            if (due.isEmpty()) return
            val title = if (due.size == 1) "Time to ${due.single().title.replaceFirstChar { it.lowercase() }}"
            else "${due.size} chores due now"
            Notifier.post(
                ctx = ctx,
                channel = Notifier.Channel.Reminders,
                key = "due:$today:" + due.joinToString(",") { it.questId.toString() },
                title = title,
                text = due.joinToString(" · ") { q -> q.title + (q.reward.let { " · $it coins" }) },
                tab = SquireNotifyWorker.TAB_TODAY,
                settings = state.notify,
                notificationId = NOTIF_BASE + SLOT_DUE_BASE,
            )
        }

        /**
         * The evening streak warning (SQUIRE-T-0141) — at most one a night, and only for a streak
         * genuinely in danger. [NotifyPolicy.streakAtRisk] holds the judgement.
         */
        private fun announceStreak(ctx: Context, state: StateView, today: LocalDate) {
            val streak = NotifyPolicy.streakAtRisk(state.streaks) ?: return
            Notifier.post(
                ctx = ctx,
                channel = Notifier.Channel.Reminders,
                key = "streak:$today",
                title = "Your ${streak.current}-day ${streak.name} streak ends at midnight",
                text = "One chore saves it.",
                tab = SquireNotifyWorker.TAB_TODAY,
                settings = state.notify,
                notificationId = NOTIF_BASE + SLOT_STREAK,
            )
        }

        /**
         * Arm one one-shot per chore time, each for its next occurrence in the household's zone.
         * Called after every state fetch, so a change made in the Keep lands within a poll.
         */
        fun schedule(ctx: Context, choreTimes: List<Int>, timezone: String?, dueTimes: List<Int> = emptyList()) {
            val wm = WorkManager.getInstance(ctx)
            val zone = zoneOf(timezone)
            // Cancel every slot first: a parent dropping from two times to one must not leave the
            // second armed forever.
            for (slot in ALL_SLOTS) wm.cancelUniqueWork(WORK_PREFIX + slot)

            fun arm(slot: Int, minutes: Int) {
                val delay = untilNext(minutes, zone)
                val request = OneTimeWorkRequestBuilder<ChoreReminderWorker>()
                    .setInitialDelay(delay.toMinutes().coerceAtLeast(1), TimeUnit.MINUTES)
                    .setInputData(androidx.work.Data.Builder().putInt(KEY_SLOT, slot).build())
                    .build()
                wm.enqueueUniqueWork(WORK_PREFIX + slot, ExistingWorkPolicy.REPLACE, request)
            }
            choreTimes.take(MAX_CHORE_SLOTS).forEachIndexed { slot, minutes -> arm(slot, minutes) }
            // One armed slot per distinct chore due time (SQUIRE-T-0142) — several chores sharing a
            // time share a slot, and are announced together.
            dueTimes.distinct().sorted().take(MAX_DUE_SLOTS).forEachIndexed { i, minutes ->
                arm(SLOT_DUE_BASE + i, minutes)
            }
            // The streak check is not a household setting: two hours before the household's midnight,
            // late enough to be a real last call and early enough to still do a chore. It is armed
            // whether or not chore times are set — the streak is the child's, not the schedule's.
            arm(SLOT_STREAK, STREAK_CHECK_MINUTE)
        }

        /** Stop every reminder (unpair, or a Knight device). */
        fun cancel(ctx: Context) {
            val wm = WorkManager.getInstance(ctx)
            for (slot in ALL_SLOTS) wm.cancelUniqueWork(WORK_PREFIX + slot)
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
        private const val MAX_CHORE_SLOTS = 2

        /** The evening streak check, scheduled like a chore time but never configured as one. */
        private const val SLOT_STREAK = 9
        private const val STREAK_CHECK_MINUTE = 22 * 60 // 22:00 household time — two hours to midnight
        /** Slots for per-chore due times, above the household ones and the streak check. */
        private const val SLOT_DUE_BASE = 10
        private const val MAX_DUE_SLOTS = 8
        private val ALL_SLOTS =
            (0 until MAX_CHORE_SLOTS) + SLOT_STREAK + (SLOT_DUE_BASE until SLOT_DUE_BASE + MAX_DUE_SLOTS)
        private const val NOTIF_BASE = 4930
    }
}
