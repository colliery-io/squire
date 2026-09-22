package com.squire.core

import com.squire.sdk.model.ClaimState
import com.squire.sdk.model.ClaimStateKind
import com.squire.sdk.model.ClaimStatus
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
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
}
