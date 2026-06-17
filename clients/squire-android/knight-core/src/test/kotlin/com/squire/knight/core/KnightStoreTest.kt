package com.squire.knight.core

import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.SquireSummary
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import java.io.IOException
import java.util.concurrent.atomic.AtomicLong
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

private fun review(balance: Int) = HouseholdReview(
    generatedAt = 1L,
    pendingClaims = emptyList(),
    pendingRequests = emptyList(),
    squires = listOf(SquireSummary(balance = balance, displayName = "Gawain", squire = 2)),
    items = emptyList(),
    quests = emptyList(),
    today = 20624,
)

/** Yields [review] until [online] flips false, then throws like an unreachable Keep. */
private class FakeFetcher(var review: HouseholdReview, var online: Boolean = true) : ReviewFetcher {
    override suspend fun fetch(): HouseholdReview =
        if (online) review else throw IOException("computer unreachable")
}

private class CountingSubmit(val result: SubmitResult = SubmitResult.Ack) : KnightSubmit {
    val submitted = mutableListOf<KnightCommand>()
    override suspend fun submit(command: KnightCommand): SubmitResult {
        submitted += command
        return result
    }
}

class KnightStoreTest {

    private val json = Json { ignoreUnknownKeys = true }

    private fun store(
        fetcher: FakeFetcher,
        submit: KnightSubmit = CountingSubmit(),
        ids: () -> Long = AtomicLong(1000)::getAndIncrement,
    ): Pair<KnightStore, PrivilegedOutbox> {
        val outbox = InMemoryPrivilegedOutbox()
        val engine = KnightSyncEngine(outbox, submit)
        return KnightStore(fetcher, InMemoryReviewCache(), outbox, engine, json, ids) to outbox
    }

    @Test
    fun `refresh renders fresh and flags not-from-cache`() = runTest {
        val (s, _) = store(FakeFetcher(review(balance = 10)))
        s.refresh()
        val ready = assertIs<KnightUiState.Ready>(s.state.value)
        assertEquals(10, ready.review.squires.single().balance)
        assertEquals(false, ready.fromCache)
    }

    @Test
    fun `refresh falls back to cache when the Keep is unreachable`() = runTest {
        val fetcher = FakeFetcher(review(balance = 10))
        val (s, _) = store(fetcher)
        s.refresh()             // caches the online view
        fetcher.online = false  // Keep goes away
        s.refresh()             // must not throw — serves cache

        val ready = assertIs<KnightUiState.Ready>(s.state.value)
        assertEquals(10, ready.review.squires.single().balance)
        assertTrue(ready.fromCache)
    }

    @Test
    fun `error when offline with no cache`() = runTest {
        val (s, _) = store(FakeFetcher(review(balance = 0), online = false))
        s.refresh()
        assertIs<KnightUiState.Error>(s.state.value)
    }

    @Test
    fun `approveClaim enqueues a review command and drains it`() = runTest {
        val submit = CountingSubmit()
        val (s, outbox) = store(FakeFetcher(review(balance = 0)), submit)
        s.approveClaim(claimId = 555)

        assertEquals(1, submit.submitted.size)
        val cmd = assertIs<KnightCommand.ReviewClaim>(submit.submitted.single())
        assertEquals(555L, cmd.req.claimId)
        assertTrue(outbox.pending().isEmpty()) // ack drained it
    }

    @Test
    fun `redeem mints a command id and survives offline on the outbox`() = runTest {
        val submit = CountingSubmit(result = SubmitResult.Offline)
        val ids = AtomicLong(7000)
        val (s, outbox) = store(FakeFetcher(review(balance = 0)), submit) { ids.getAndIncrement() }
        s.redeem(squire = 2, itemId = 200)

        val pending = assertIs<KnightCommand.Redeem>(outbox.pending().single())
        assertEquals(7000L, pending.req.commandId) // minted id retained for retry
        assertEquals(200L, pending.req.itemId)
    }

    @Test
    fun `adjust rejects a blank reason before queuing`() = runTest {
        val submit = CountingSubmit()
        val (s, outbox) = store(FakeFetcher(review(balance = 0)), submit)
        var threw = false
        try {
            s.adjust(squire = 2, amount = 5, reason = "   ")
        } catch (e: IllegalArgumentException) {
            threw = true
        }
        assertTrue(threw)
        assertTrue(outbox.pending().isEmpty())
        assertTrue(submit.submitted.isEmpty())
    }
}
