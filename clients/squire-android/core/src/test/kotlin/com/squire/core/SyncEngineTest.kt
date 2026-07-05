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

/** Records every call; answers each submission via [resultFor] (defaults to Ack). */
private class RecordingApi(
    private val resultFor: (Long) -> SubmitResult = { SubmitResult.Ack },
) : SubmissionApi {
    val claimIds = mutableListOf<Long>()
    val requestIds = mutableListOf<Long>()

    override suspend fun submitClaim(req: SubmitClaimReq): SubmitResult {
        claimIds += req.claimId
        return resultFor(req.claimId)
    }

    override suspend fun requestRedemption(req: RequestRedemptionReq): SubmitResult {
        requestIds += req.requestId
        return resultFor(req.requestId)
    }

    override suspend fun requestCashOut(req: com.squire.sdk.model.RequestCashOutReq): SubmitResult {
        requestIds += req.requestId
        return resultFor(req.requestId)
    }
}

/** Violates the never-throws contract, to prove the engine's belt-and-braces catch (NFR-1). */
private class ThrowingApi : SubmissionApi {
    override suspend fun submitClaim(req: SubmitClaimReq): SubmitResult =
        throw IOException("computer unreachable")

    override suspend fun requestRedemption(req: RequestRedemptionReq): SubmitResult =
        throw IOException("computer unreachable")

    override suspend fun requestCashOut(req: com.squire.sdk.model.RequestCashOutReq): SubmitResult =
        throw IOException("computer unreachable")
}

private class FakeStatePort(private val resolved: ResolvedIds) : StatePort {
    override suspend fun resolvedIds(): ResolvedIds = resolved
}

private val NOTHING_RESOLVED = ResolvedIds(emptySet(), emptySet(), emptySet())

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
        val statePort = FakeStatePort(ResolvedIds(claims = setOf(1L), requests = setOf(2L), cashouts = emptySet()))
        val engine = SyncEngine(outbox, api, statePort)

        val outcome = engine.sync()

        assertEquals(listOf(1L), api.claimIds)
        assertEquals(listOf(2L), api.requestIds)
        assertTrue(outbox.pending().isEmpty())
        val synced = assertIs<SyncOutcome.Synced>(outcome)
        assertEquals(2, synced.resolved)
        assertEquals(0, synced.dropped)
        assertEquals(0, synced.remaining)
    }

    @Test
    fun `sync offline keeps items pending without throwing`() = runTest {
        val outbox = InMemoryOutbox()
        outbox.enqueue(claim(1))
        outbox.enqueue(claim(2))
        val api = RecordingApi(resultFor = { SubmitResult.Offline })
        val engine = SyncEngine(outbox, api, FakeStatePort(NOTHING_RESOLVED))

        val outcome = engine.sync()

        assertIs<SyncOutcome.Offline>(outcome)
        assertEquals(2, outbox.pending().size)
    }

    @Test
    fun `an api that throws despite its contract still cannot crash a sync pass`() = runTest {
        val outbox = InMemoryOutbox()
        outbox.enqueue(claim(1))
        val engine = SyncEngine(outbox, ThrowingApi(), FakeStatePort(NOTHING_RESOLVED))

        val outcome = engine.sync()

        assertIs<SyncOutcome.Offline>(outcome)
        assertEquals(1, outbox.pending().size)
    }

    @Test
    fun `reconciliation removes only ids present in refreshed state`() = runTest {
        val outbox = InMemoryOutbox()
        outbox.enqueue(claim(1))
        outbox.enqueue(claim(2))
        val api = RecordingApi()
        val statePort = FakeStatePort(ResolvedIds(claims = setOf(1L), requests = emptySet(), cashouts = emptySet()))
        val engine = SyncEngine(outbox, api, statePort)

        val outcome = engine.sync()

        val remainingIds = outbox.pending().map { it.id }
        assertEquals(listOf(2L), remainingIds)
        val synced = assertIs<SyncOutcome.Synced>(outcome)
        assertEquals(1, synced.resolved)
        assertEquals(1, synced.remaining)
    }

    @Test
    fun `a rejected claim is dropped and does not block items queued behind it`() = runTest {
        // Regression: SQUIRE-T-0128. A 409 already-claimed poison pill used to abort the whole
        // flush pass, so the claim behind it was never even sent.
        val outbox = InMemoryOutbox()
        outbox.enqueue(claim(1)) // poison: server says already claimed today
        outbox.enqueue(claim(2)) // fresh claim queued behind it
        val rejections = mutableListOf<Long>()
        val api = RecordingApi(resultFor = { id ->
            if (id == 1L) SubmitResult.Rejected(409, "AlreadyClaimedToday") else SubmitResult.Ack
        })
        val statePort = FakeStatePort(ResolvedIds(claims = setOf(2L), requests = emptySet(), cashouts = emptySet()))
        val engine = SyncEngine(outbox, api, statePort, onRejected = { item, _ -> rejections += item.id })

        val outcome = engine.sync()

        assertEquals(listOf(1L, 2L), api.claimIds) // the fresh claim WAS sent
        assertTrue(outbox.pending().isEmpty()) // poison dropped, fresh claim resolved
        assertEquals(listOf(1L), rejections)
        val synced = assertIs<SyncOutcome.Synced>(outcome)
        assertEquals(1, synced.resolved)
        assertEquals(1, synced.dropped)
        assertEquals(0, synced.remaining)
    }

    @Test
    fun `going offline mid-drain still drops already-rejected items but keeps the rest`() = runTest {
        val outbox = InMemoryOutbox()
        outbox.enqueue(claim(1)) // rejected
        outbox.enqueue(claim(2)) // offline from here on
        outbox.enqueue(claim(3)) // never reached
        val api = RecordingApi(resultFor = { id ->
            when (id) {
                1L -> SubmitResult.Rejected(409, "AlreadyClaimedToday")
                else -> SubmitResult.Offline
            }
        })
        val engine = SyncEngine(outbox, api, FakeStatePort(NOTHING_RESOLVED))

        val outcome = engine.sync()

        assertIs<SyncOutcome.Offline>(outcome)
        assertEquals(listOf(2L, 3L), outbox.pending().map { it.id })
    }

    @Test
    fun `state cache round-trips a blob`() {
        val cache = InMemoryStateCache()
        assertEquals(null, cache.load())
        cache.save("""{"my_claims":[1,2]}""")
        assertEquals("""{"my_claims":[1,2]}""", cache.load())
    }
}
