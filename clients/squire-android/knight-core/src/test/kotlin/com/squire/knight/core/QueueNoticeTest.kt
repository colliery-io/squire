package com.squire.knight.core

import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.PendingCashOut
import com.squire.sdk.model.PendingClaim
import com.squire.sdk.model.PendingRequest
import com.squire.sdk.model.SquireSummary
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * The Knight queue notice (SQUIRE-T-0139). The behaviour worth protecting is the *silence*: a queue
 * left unsealed must not re-announce itself on every poll.
 */
class QueueNoticeTest {

    private fun review(
        claims: List<PendingClaim> = emptyList(),
        requests: List<PendingRequest> = emptyList(),
        cashouts: List<PendingCashOut> = emptyList(),
    ) = HouseholdReview(
        generatedAt = 1L,
        pendingClaims = claims,
        pendingRequests = requests,
        pendingCashouts = cashouts,
        squires = listOf(
            SquireSummary(balance = 12, displayName = "Matrim", squire = 2),
            SquireSummary(balance = 3, displayName = "Perrin", squire = 3),
        ),
        items = emptyList(),
        quests = emptyList(),
        today = 20624,
    )

    private fun claim(id: Long, squire: Long, title: String) =
        PendingClaim(claimId = id, on = 20624, questTitle = title, squire = squire)

    @Test
    fun `an empty queue says nothing`() {
        assertNull(QueueNotice.notice(review(), emptySet()))
    }

    @Test
    fun `one waiting item names the squire and the chore`() {
        val n = QueueNotice.notice(review(claims = listOf(claim(1, 2, "Tidy your room"))), emptySet())!!
        assertEquals("Matrim turned in Tidy your room", n.title)
        assertEquals(listOf("waiting:claim:1"), n.keys)
    }

    @Test
    fun `a queue already announced is silent on the next poll`() {
        val r = review(claims = listOf(claim(1, 2, "Tidy your room")))
        val first = QueueNotice.notice(r, emptySet())!!
        // The parent has not sealed it; fifteen minutes later we must not say it again.
        assertNull(QueueNotice.notice(r, first.keys.toSet()))
    }

    @Test
    fun `a new arrival reports the whole queue, not just itself`() {
        val spoken = setOf("waiting:claim:1")
        val r = review(claims = listOf(claim(1, 2, "Tidy your room"), claim(2, 3, "Walk the dog")))
        val n = QueueNotice.notice(r, spoken)!!
        assertEquals("Perrin turned in Walk the dog", n.title)
        assertEquals("2 waiting for your seal.", n.text, "the count is the whole queue")
        assertEquals(listOf("waiting:claim:2"), n.keys, "only the new item is marked spoken")
    }

    @Test
    fun `several new at once summarise`() {
        val r = review(claims = listOf(claim(1, 2, "Tidy your room"), claim(2, 3, "Walk the dog")))
        val n = QueueNotice.notice(r, emptySet())!!
        assertEquals("2 waiting for your seal", n.title)
        assertTrue(n.text.contains("Matrim: Tidy your room"))
        assertTrue(n.text.contains("Perrin: Walk the dog"))
    }

    @Test
    fun `reward requests and cash-outs count as waiting too`() {
        val r = review(
            requests = listOf(PendingRequest(cost = 15, itemName = "Movie night", requestId = 7, squire = 2)),
            cashouts = listOf(PendingCashOut(amount = 5, requestId = 8, squire = 2)),
        )
        val items = QueueNotice.waiting(r)
        assertEquals(2, items.size, "the count must match what the Review screen shows")
        assertTrue(items.any { it.key == "waiting:request:7" })
        assertTrue(items.any { it.key == "waiting:cashout:8" })
    }

    @Test
    fun `an unknown squire degrades to a phrase, never to an id`() {
        val r = review(claims = listOf(claim(1, 99, "Tidy your room")))
        assertEquals("A squire turned in Tidy your room", QueueNotice.notice(r, emptySet())!!.title)
    }
}
