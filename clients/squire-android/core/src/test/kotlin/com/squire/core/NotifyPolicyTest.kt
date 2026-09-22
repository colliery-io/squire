package com.squire.core

import com.squire.sdk.model.ClaimState
import com.squire.sdk.model.ClaimStateKind
import com.squire.sdk.model.ClaimStatus
import com.squire.sdk.model.QuestCard
import com.squire.sdk.model.QuestStatus
import com.squire.sdk.model.StreakView
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * The decision half of notifications (SQUIRE-T-0138). These are the tests that actually protect a
 * child's evening: posting is untestable here, but *whether* to post is all decided in this file.
 */
class NotifyPolicyTest {

    // ── quiet hours ────────────────────────────────────────────────────────────────────────────

    @Test
    fun `waking window keeps the night quiet`() {
        val from = 7 * 60
        val to = 20 * 60
        assertFalse(NotifyPolicy.inWakingHours(6 * 60 + 59, from, to), "06:59 is still the night")
        assertTrue(NotifyPolicy.inWakingHours(7 * 60, from, to), "07:00 is the first waking minute")
        assertTrue(NotifyPolicy.inWakingHours(19 * 60 + 59, from, to))
        assertFalse(NotifyPolicy.inWakingHours(20 * 60, from, to), "20:00 is quiet — the window is half-open")
        assertFalse(NotifyPolicy.inWakingHours(2 * 60, from, to))
    }

    @Test
    fun `a window that wraps past midnight is read as spanning it`() {
        // 20:00 → 07:00: an odd thing to ask for, but a legible one.
        val from = 20 * 60
        val to = 7 * 60
        assertTrue(NotifyPolicy.inWakingHours(22 * 60, from, to))
        assertTrue(NotifyPolicy.inWakingHours(2 * 60, from, to))
        assertFalse(NotifyPolicy.inWakingHours(12 * 60, from, to))
    }

    @Test
    fun `a nonsense window falls back to the default rather than silencing the day`() {
        assertTrue(NotifyPolicy.inWakingHours(12 * 60, -5, 9999), "midday is never quiet by accident")
        assertFalse(NotifyPolicy.inWakingHours(3 * 60, -5, 9999))
        assertTrue(NotifyPolicy.inWakingHours(3 * 60, 600, 600), "a zero-width window means no quiet hours")
    }

    // ── outcomes ───────────────────────────────────────────────────────────────────────────────

    private fun claim(id: Long, title: String, kind: ClaimStateKind, points: Int? = null, reason: String? = null) =
        ClaimStatus(claimId = id, on = 1, questTitle = title, state = ClaimState(state = kind, points = points, reason = reason))

    @Test
    fun `the first sight of the world says nothing`() {
        val now = listOf(claim(1, "Tidy your room", ClaimStateKind.Approved, points = 10))
        assertEquals(emptyList(), NotifyPolicy.outcomes(previous = null, current = now))
    }

    @Test
    fun `a seal and a not-yet are each announced once`() {
        val pending = listOf(
            claim(1, "Tidy your room", ClaimStateKind.Pending),
            claim(2, "Practice piano", ClaimStateKind.Pending),
        )
        val before = NotifyPolicy.markers(pending)

        val after = listOf(
            claim(1, "Tidy your room", ClaimStateKind.Approved, points = 10),
            claim(2, "Practice piano", ClaimStateKind.Rejected, reason = "Play it through once more"),
        )
        val out = NotifyPolicy.outcomes(before, after)
        assertEquals(2, out.size)

        val sealed = out.single { it.sealed }
        assertEquals("sealed:1", sealed.key)
        assertEquals("Tidy your room", sealed.questTitle)
        assertEquals(10, sealed.coins)

        val notYet = out.single { !it.sealed }
        assertEquals("notyet:2", notYet.key)
        assertEquals("Play it through once more", notYet.reason)

        // Polling again with the same world is silent.
        assertEquals(emptyList(), NotifyPolicy.outcomes(NotifyPolicy.markers(after), after))
    }

    @Test
    fun `a claim still waiting says nothing`() {
        val before = NotifyPolicy.markers(emptyList())
        val after = listOf(claim(3, "Feed the dog", ClaimStateKind.Pending))
        assertEquals(emptyList(), NotifyPolicy.outcomes(before, after))
    }

    @Test
    fun `a blank rejection reason is dropped rather than shown as empty`() {
        val before = NotifyPolicy.markers(listOf(claim(4, "Homework", ClaimStateKind.Pending)))
        val after = listOf(claim(4, "Homework", ClaimStateKind.Rejected, reason = "   "))
        assertEquals(null, NotifyPolicy.outcomes(before, after).single().reason)
    }

    // ── coins ──────────────────────────────────────────────────────────────────────────────────

    @Test
    fun `coins explained by a seal are not announced twice`() {
        val sealed = listOf(NotifyPolicy.Outcome("sealed:1", "Tidy your room", sealed = true, coins = 10))
        assertEquals(0, NotifyPolicy.unexplainedCoinGain(previousBalance = 5, currentBalance = 15, sealed = sealed))
        // A grown-up granted 3 on top of the sealed 10.
        assertEquals(3, NotifyPolicy.unexplainedCoinGain(previousBalance = 5, currentBalance = 18, sealed = sealed))
        // A plain grant, no seals at all.
        assertEquals(7, NotifyPolicy.unexplainedCoinGain(previousBalance = 5, currentBalance = 12, sealed = emptyList()))
    }

    @Test
    fun `a spend or an unknown baseline never announces coins`() {
        assertEquals(0, NotifyPolicy.unexplainedCoinGain(previousBalance = 20, currentBalance = 5, sealed = emptyList()))
        assertEquals(0, NotifyPolicy.unexplainedCoinGain(previousBalance = -1, currentBalance = 5, sealed = emptyList()))
    }

    // ── de-duplication ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `a key already spoken is not spoken again`() {
        val spoken = setOf("sealed:1", "reminder:2026-09-22:0")
        assertEquals(
            listOf("sealed:2"),
            NotifyPolicy.unspoken(listOf("sealed:1", "sealed:2", "reminder:2026-09-22:0"), spoken),
        )
    }

    // ── the chore-time nudge (SQUIRE-T-0140) ───────────────────────────────────────────────────

    private fun quest(id: Long, title: String, status: QuestStatus) =
        QuestCard(on = 1, questId = id, reward = 5, status = status, title = title)

    @Test
    fun `nothing left means nothing said`() {
        // The rule that keeps the reminder worth reading.
        assertEquals(null, NotifyPolicy.choreReminder(emptyList()))
        assertEquals(
            null,
            NotifyPolicy.choreReminder(
                listOf(
                    quest(1, "Tidy your room", QuestStatus.CompletedToday),
                    quest(2, "Feed the dog", QuestStatus.Pending),
                    quest(3, "Walk the dog", QuestStatus.TakenByOther),
                ),
            ),
            "sealed, waiting on a grown-up, and a sibling's chore are all 'not left'",
        )
    }

    @Test
    fun `what is left is named`() {
        val one = NotifyPolicy.choreReminder(listOf(quest(1, "Tidy your room", QuestStatus.Available)))!!
        assertEquals("1 chore left today", one.first)
        assertEquals("Tidy your room", one.second)

        val two = NotifyPolicy.choreReminder(
            listOf(
                quest(1, "Tidy your room", QuestStatus.Available),
                quest(2, "Practice piano", QuestStatus.Available),
                quest(3, "Feed the dog", QuestStatus.CompletedToday),
            ),
        )!!
        assertEquals("2 chores left today", two.first)
        assertTrue(two.second.contains("Tidy your room") && two.second.contains("Practice piano"))
    }

    @Test
    fun `the next chore time is today when ahead and tomorrow once passed`() {
        assertEquals(90, NotifyPolicy.minutesUntilNext(minuteOfDay = 16 * 60 + 30, nowMinuteOfDay = 15 * 60))
        assertEquals(
            23 * 60 + 30,
            NotifyPolicy.minutesUntilNext(minuteOfDay = 16 * 60 + 30, nowMinuteOfDay = 17 * 60),
            "already gone today, so tomorrow",
        )
        assertEquals(24 * 60, NotifyPolicy.minutesUntilNext(minuteOfDay = 600, nowMinuteOfDay = 600), "never zero")
    }

    // ── streak at risk (SQUIRE-T-0141) ─────────────────────────────────────────────────────────

    private fun streak(name: String, current: Int, alive: Boolean = true, covered: Boolean? = false) =
        StreakView(alive = alive, best = 9, current = current, name = name, coveredToday = covered)

    @Test
    fun `only a live streak with something to lose is warned about`() {
        assertEquals("Room Master", NotifyPolicy.streakAtRisk(listOf(streak("Room Master", 5)))?.name)
        assertNull(NotifyPolicy.streakAtRisk(listOf(streak("Room Master", 1))), "one day is not yet a streak")
        assertNull(NotifyPolicy.streakAtRisk(listOf(streak("Room Master", 5, alive = false))), "already broken")
        assertNull(NotifyPolicy.streakAtRisk(emptyList()))
    }

    @Test
    fun `a chore already done or awaiting a seal is not at risk`() {
        assertNull(
            NotifyPolicy.streakAtRisk(listOf(streak("Room Master", 5, covered = true))),
            "the child has acted; whether it is sealed in time is a parent's business",
        )
    }

    @Test
    fun `an older server that cannot say is treated as at risk`() {
        // coveredToday = null means "this server does not know"; warning is the safer error.
        assertEquals("Room Master", NotifyPolicy.streakAtRisk(listOf(streak("Room Master", 5, covered = null)))?.name)
    }

    @Test
    fun `of several at risk the longest is chosen`() {
        val pick = NotifyPolicy.streakAtRisk(
            listOf(streak("Early Bird", 3), streak("Room Master", 7), streak("Piano", 2)),
        )
        assertEquals("Room Master", pick?.name)
    }

    // ── per-chore due times (SQUIRE-T-0142) ────────────────────────────────────────────────────

    private fun timed(id: Long, title: String, due: Int?, status: QuestStatus = QuestStatus.Available) =
        QuestCard(on = 1, questId = id, reward = 3, status = status, title = title, dueTime = due)

    @Test
    fun `timed chores sort ahead of untimed ones`() {
        val sorted = NotifyPolicy.byDueTime(
            listOf(
                timed(1, "Homework", 18 * 60),
                timed(2, "Anytime chore", null),
                timed(3, "Brush teeth", 7 * 60 + 30),
            ),
        )
        assertEquals(listOf("Brush teeth", "Homework", "Anytime chore"), sorted.map { it.title })
    }

    @Test
    fun `only undone chores near the time count as due now`() {
        val quests = listOf(
            timed(1, "Brush teeth", 7 * 60 + 30),
            timed(2, "Homework", 18 * 60),
            timed(3, "Already done", 7 * 60 + 30, status = QuestStatus.CompletedToday),
            timed(4, "Anytime", null),
        )
        val now = NotifyPolicy.dueNow(quests, minuteOfDay = 7 * 60 + 30)
        assertEquals(listOf("Brush teeth"), now.map { it.title }, "done, far-off and untimed all excluded")
        assertTrue(NotifyPolicy.dueNow(quests, minuteOfDay = 12 * 60).isEmpty(), "midday has nothing due")
    }

    @Test
    fun `several due at once are collected, to be said in one breath`() {
        val quests = listOf(timed(1, "Brush teeth", 450), timed(2, "Make bed", 455))
        assertEquals(2, NotifyPolicy.dueNow(quests, minuteOfDay = 452).size)
    }
}
