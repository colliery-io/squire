package com.squire.core

import com.squire.sdk.model.RequestRedemptionReq
import com.squire.sdk.model.SubmitClaimReq
import kotlinx.coroutines.test.runTest
import java.io.IOException
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

private fun claim(id: Long, quest: Long = 100, on: Int = 1) =
    OutboxItem.Claim(SubmitClaimReq(claimId = id, on = on, questId = quest))

private fun redemption(id: Long, item: Long = 200) =
    OutboxItem.Redemption(RequestRedemptionReq(itemId = item, requestId = id))

/** Records every call; succeeds unless [failWith] is set. */
private class RecordingApi(private val failWith: Throwable? = null) : SubmissionApi {
    val claimIds = mutableListOf<Long>()
    val requestIds = mutableListOf<Long>()

    override suspend fun submitClaim(req: SubmitClaimReq) {
        failWith?.let { throw it }
        claimIds += req.claimId
    }

    override suspend fun requestRedemption(req: RequestRedemptionReq) {
        failWith?.let { throw it }
        requestIds += req.requestId
    }
}

private class FakeStatePort(private val resolved: ResolvedIds) : StatePort {
    override suspend fun resolvedIds(): ResolvedIds = resolved
}

class SyncEngineTest {

    @Test
    fun `enqueueing same claim id twice is idempotent`() {
        val outbox = InMemoryOutbox()
        outbox.enqueue(claim(42))
        outbox.enqueue(claim(42))
        assertEquals(1, outbox.pending().size)
        assertEquals(42L, outbox.pending().single().id)
    }

    @Test
    fun `sync flushes pending and reconciles resolved items out`() = runTest {
        val outbox = InMemoryOutbox()
        outbox.enqueue(claim(1))
        outbox.enqueue(redemption(2))
        val api = RecordingApi()
        val statePort = FakeStatePort(ResolvedIds(claims = setOf(1L), requests = setOf(2L)))
        val engine = SyncEngine(outbox, api, statePort)

        val outcome = engine.sync()

        assertEquals(listOf(1L), api.claimIds)
        assertEquals(listOf(2L), api.requestIds)
        assertTrue(outbox.pending().isEmpty())
        val synced = assertIs<SyncOutcome.Synced>(outcome)
        assertEquals(2, synced.resolved)
        assertEquals(0, synced.remaining)
    }

    @Test
    fun `sync offline keeps items pending without throwing`() = runTest {
        val outbox = InMemoryOutbox()
        outbox.enqueue(claim(1))
        outbox.enqueue(claim(2))
        val api = RecordingApi(failWith = IOException("computer unreachable"))
        val statePort = FakeStatePort(ResolvedIds(emptySet(), emptySet()))
        val engine = SyncEngine(outbox, api, statePort)

        val outcome = engine.sync()

        assertIs<SyncOutcome.Offline>(outcome)
        assertEquals(2, outbox.pending().size)
    }

    @Test
    fun `reconciliation removes only ids present in refreshed state`() = runTest {
        val outbox = InMemoryOutbox()
        outbox.enqueue(claim(1))
        outbox.enqueue(claim(2))
        val api = RecordingApi()
        val statePort = FakeStatePort(ResolvedIds(claims = setOf(1L), requests = emptySet()))
        val engine = SyncEngine(outbox, api, statePort)

        val outcome = engine.sync()

        val remainingIds = outbox.pending().map { it.id }
        assertEquals(listOf(2L), remainingIds)
        val synced = assertIs<SyncOutcome.Synced>(outcome)
        assertEquals(1, synced.resolved)
        assertEquals(1, synced.remaining)
    }

    @Test
    fun `state cache round-trips a blob`() {
        val cache = InMemoryStateCache()
        assertEquals(null, cache.load())
        cache.save("""{"my_claims":[1,2]}""")
        assertEquals("""{"my_claims":[1,2]}""", cache.load())
    }
}
