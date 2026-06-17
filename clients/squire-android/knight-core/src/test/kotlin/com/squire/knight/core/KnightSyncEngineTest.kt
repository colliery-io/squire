package com.squire.knight.core

import com.squire.sdk.model.AdjustReq
import com.squire.sdk.model.DecisionDto
import com.squire.sdk.model.DecisionKind
import com.squire.sdk.model.ReviewClaimReq
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

private fun approveClaim(id: Long) =
    KnightCommand.ReviewClaim(ReviewClaimReq(claimId = id, decision = DecisionDto(DecisionKind.approve)))

private fun adjust(commandId: Long, squire: Long = 2, amount: Long = 5, reason: String = "chore") =
    KnightCommand.Adjust(AdjustReq(amount = amount, commandId = commandId, reason = reason, squire = squire))

/** Returns a programmable [SubmitResult] per command key; records the order of submitted keys. */
private class FakeSubmit(private val results: Map<String, SubmitResult> = emptyMap(),
                         private val default: SubmitResult = SubmitResult.Ack) : KnightSubmit {
    val submitted = mutableListOf<String>()
    override suspend fun submit(command: KnightCommand): SubmitResult {
        submitted += command.key
        return results[command.key] ?: default
    }
}

class KnightSyncEngineTest {

    @Test
    fun `enqueueing the same command key twice is idempotent`() {
        val outbox = InMemoryPrivilegedOutbox()
        outbox.enqueue(approveClaim(42))
        outbox.enqueue(approveClaim(42))
        assertEquals(1, outbox.pending().size)
        assertEquals("REVIEW_CLAIM:42", outbox.pending().single().key)
    }

    @Test
    fun `same numeric id under different kinds does not collide`() {
        val outbox = InMemoryPrivilegedOutbox()
        outbox.enqueue(approveClaim(7))   // REVIEW_CLAIM:7
        outbox.enqueue(adjust(7))         // ADJUST:7
        assertEquals(2, outbox.pending().size)
    }

    @Test
    fun `ack resolves items out of the outbox`() = runTest {
        val outbox = InMemoryPrivilegedOutbox()
        outbox.enqueue(approveClaim(1))
        outbox.enqueue(adjust(2))
        val submit = FakeSubmit(default = SubmitResult.Ack)
        val outcome = KnightSyncEngine(outbox, submit).sync()

        assertEquals(listOf("REVIEW_CLAIM:1", "ADJUST:2"), submit.submitted)
        assertTrue(outbox.pending().isEmpty())
        val synced = assertIs<SyncOutcome.Synced>(outcome)
        assertEquals(2, synced.acked)
        assertEquals(0, synced.dropped)
        assertEquals(0, synced.remaining)
    }

    @Test
    fun `already-done is treated as resolved (no double-action)`() = runTest {
        val outbox = InMemoryPrivilegedOutbox()
        outbox.enqueue(approveClaim(1))
        val submit = FakeSubmit(results = mapOf("REVIEW_CLAIM:1" to SubmitResult.AlreadyDone))
        val outcome = KnightSyncEngine(outbox, submit).sync()

        assertTrue(outbox.pending().isEmpty())
        assertEquals(1, assertIs<SyncOutcome.Synced>(outcome).acked)
    }

    @Test
    fun `offline stops the drain and keeps the unsent items queued`() = runTest {
        val outbox = InMemoryPrivilegedOutbox()
        outbox.enqueue(approveClaim(1)) // acks
        outbox.enqueue(adjust(2))       // goes offline here
        outbox.enqueue(adjust(3))       // never attempted
        val submit = FakeSubmit(results = mapOf("ADJUST:2" to SubmitResult.Offline))
        val outcome = KnightSyncEngine(outbox, submit).sync()

        assertIs<SyncOutcome.Offline>(outcome)
        // The acked one is gone; the offline one and everything after stay queued.
        assertEquals(listOf("ADJUST:2", "ADJUST:3"), outbox.pending().map { it.key })
        assertEquals(listOf("REVIEW_CLAIM:1", "ADJUST:2"), submit.submitted) // 3 never tried
    }

    @Test
    fun `poison is dropped and reported, never retried`() = runTest {
        val outbox = InMemoryPrivilegedOutbox()
        outbox.enqueue(adjust(9))
        val submit = FakeSubmit(results = mapOf("ADJUST:9" to SubmitResult.Poison(400, "blank reason")))
        val poisoned = mutableListOf<String>()
        val outcome = KnightSyncEngine(outbox, submit) { cmd, _ -> poisoned += cmd.key }.sync()

        assertTrue(outbox.pending().isEmpty())
        assertEquals(listOf("ADJUST:9"), poisoned)
        val synced = assertIs<SyncOutcome.Synced>(outcome)
        assertEquals(0, synced.acked)
        assertEquals(1, synced.dropped)
    }
}
