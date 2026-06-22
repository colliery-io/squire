package com.squire.core

/** Result of a [SyncEngine.sync] pass. */
sealed interface SyncOutcome {
    /** A full flush + reconcile completed. */
    data class Synced(val resolved: Int, val remaining: Int) : SyncOutcome

    /** The computer was unreachable; the outbox was left intact. */
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
 * Any network failure (a throw from [api] or [statePort]) is caught: the outbox is
 * left untouched and [SyncOutcome.Offline] is returned. Nothing ever propagates,
 * so an unreachable computer can never crash the phone (NFR-1/6).
 */
class SyncEngine(
    private val outbox: Outbox,
    private val api: SubmissionApi,
    private val statePort: StatePort,
) {
    suspend fun sync(): SyncOutcome {
        return try {
            val pending = outbox.pending()
            for (item in pending) {
                when (item) {
                    is OutboxItem.Claim -> api.submitClaim(item.req)
                    is OutboxItem.Redemption -> api.requestRedemption(item.req)
                    is OutboxItem.CashOut -> api.requestCashOut(item.req)
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

            outbox.markResolved(toResolve)

            SyncOutcome.Synced(resolved = toResolve.size, remaining = outbox.pending().size)
        } catch (e: Exception) {
            SyncOutcome.Offline
        }
    }
}
