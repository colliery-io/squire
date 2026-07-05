package com.squire.core

/** Result of a [SyncEngine.sync] pass. */
sealed interface SyncOutcome {
    /**
     * A full flush + reconcile completed. [resolved] items the server acknowledged, [dropped]
     * were permanently rejected (removed, never retried), [remaining] are still pending.
     */
    data class Synced(val resolved: Int, val dropped: Int, val remaining: Int) : SyncOutcome

    /** The computer was unreachable; unresolved items were left queued for the next pass. */
    data object Offline : SyncOutcome
}

/**
 * Offline-first sync: flush-then-refetch with reconciliation by id (REQ-SY1–SY5).
 *
 * Each [sync] pass:
 *  1. flushes every pending item via the matching [api] call (server-side idempotent);
 *  2. refetches the server's resolved ids via [statePort];
 *  3. removes pending items whose ids now appear in the refreshed state.
 *
 * Flushing is per-item on the [SubmitResult] (mirrors `KnightSyncEngine`):
 *  - [SubmitResult.Ack] → resolved by the reconcile step (its id appears in refreshed state);
 *  - [SubmitResult.Rejected] → dropped immediately. A terminal 4xx (e.g. 409 already-claimed)
 *    can never succeed; before this, one rejected claim was retried forever and — because the
 *    flush is sequential — blocked every submission enqueued after it (SQUIRE-T-0128);
 *  - [SubmitResult.Offline] → stop draining, commit what's already dropped, leave the rest
 *    queued, and report [SyncOutcome.Offline].
 *
 * [api] never throws per its contract, but everything here is belt-and-braces wrapped anyway:
 * an unexpected throw (including from [statePort]) leaves the outbox intact and returns
 * [SyncOutcome.Offline], so an unreachable computer can never crash the phone (NFR-1/6).
 *
 * @param onRejected invoked for each dropped submission so the app can surface/log it.
 */
class SyncEngine(
    private val outbox: Outbox,
    private val api: SubmissionApi,
    private val statePort: StatePort,
    private val onRejected: (OutboxItem, SubmitResult.Rejected) -> Unit = { _, _ -> },
) {
    suspend fun sync(): SyncOutcome {
        return try {
            val pending = outbox.pending()
            val rejected = mutableSetOf<Long>()

            for (item in pending) {
                val result = when (item) {
                    is OutboxItem.Claim -> api.submitClaim(item.req)
                    is OutboxItem.Redemption -> api.requestRedemption(item.req)
                    is OutboxItem.CashOut -> api.requestCashOut(item.req)
                }
                when (result) {
                    SubmitResult.Ack -> Unit

                    is SubmitResult.Rejected -> {
                        rejected += item.id
                        onRejected(item, result)
                    }

                    SubmitResult.Offline -> {
                        // Reachability lost — drop what's already terminally rejected so it can't
                        // poison the next pass, leave everything else queued.
                        outbox.markResolved(rejected)
                        return SyncOutcome.Offline
                    }
                }
            }

            val resolved = statePort.resolvedIds()

            val toResolve = pending
                .filter { item ->
                    when (item.kind) {
                        SubmissionKind.CLAIM -> item.id in resolved.claims
                        SubmissionKind.REDEMPTION -> item.id in resolved.requests
                        SubmissionKind.CASHOUT -> item.id in resolved.cashouts
                    }
                }
                .map { it.id }
                .toSet()

            outbox.markResolved(toResolve + rejected)

            SyncOutcome.Synced(
                resolved = toResolve.size,
                dropped = rejected.size,
                remaining = outbox.pending().size,
            )
        } catch (e: Exception) {
            SyncOutcome.Offline
        }
    }
}
