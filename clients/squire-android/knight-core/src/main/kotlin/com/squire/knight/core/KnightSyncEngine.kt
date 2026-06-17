package com.squire.knight.core

/** Result of a [KnightSyncEngine.sync] pass. */
sealed interface SyncOutcome {
    /**
     * A drain pass completed without going offline. [acked] items committed, [dropped] were poison
     * (non-retryable, removed), [remaining] are still pending afterwards.
     */
    data class Synced(val acked: Int, val dropped: Int, val remaining: Int) : SyncOutcome

    /** The Keep went unreachable mid-drain; the outbox was left intact from that point on. */
    data object Offline : SyncOutcome
}

/**
 * Ack-based offline sync for the Knight's **privileged** outbox (REQ-K7/K8/K9).
 *
 * Unlike the Squire's reconcile-by-id-in-state, privileged commands are synchronous and terminal —
 * the Keep validates through `Engine::handle` and commits *before* acking — so an item resolves on
 * its [SubmitResult], not by polling refreshed state:
 *  - [SubmitResult.Ack] / [SubmitResult.AlreadyDone] → resolved (drop from outbox);
 *  - [SubmitResult.Poison] → dropped too (a 400/404 can never succeed; counted separately + logged);
 *  - [SubmitResult.Offline] → stop draining immediately, leaving this and later items queued.
 *
 * Each command is server-idempotent on its client-minted id, so a blind retry of an item already
 * applied returns [SubmitResult.Ack] (empty change set) or [SubmitResult.AlreadyDone] — never a
 * double-effect. Nothing here throws ([KnightSubmit] never throws), so an unreachable Keep can't
 * crash the app (NFR-1).
 *
 * @param onPoison invoked for each dropped poison command so the app can surface/log it.
 */
class KnightSyncEngine(
    private val outbox: PrivilegedOutbox,
    private val submit: KnightSubmit,
    private val onPoison: (KnightCommand, SubmitResult.Poison) -> Unit = { _, _ -> },
) {
    suspend fun sync(): SyncOutcome {
        val resolved = mutableSetOf<String>()
        var acked = 0
        var dropped = 0

        for (command in outbox.pending()) {
            when (val result = submit.submit(command)) {
                SubmitResult.Ack, SubmitResult.AlreadyDone -> {
                    resolved += command.key
                    acked++
                }

                is SubmitResult.Poison -> {
                    resolved += command.key
                    dropped++
                    onPoison(command, result)
                }

                SubmitResult.Offline -> {
                    // Reachability lost — commit what resolved so far, leave the rest queued.
                    outbox.markResolved(resolved)
                    return SyncOutcome.Offline
                }
            }
        }

        outbox.markResolved(resolved)
        return SyncOutcome.Synced(acked = acked, dropped = dropped, remaining = outbox.pending().size)
    }
}
