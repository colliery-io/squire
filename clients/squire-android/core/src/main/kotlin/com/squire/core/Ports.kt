package com.squire.core

import com.squire.sdk.model.RequestCashOutReq
import com.squire.sdk.model.RequestRedemptionReq
import com.squire.sdk.model.SubmitClaimReq

/**
 * Outcome of posting one submission to the computer (mirrors the Knight side's `SubmitResult`).
 *
 * The split matters to the [SyncEngine]: [Rejected] is terminal — the server said this exact
 * submission can never succeed (e.g. 409 already-claimed-today), so retrying it forever would
 * wedge the queue and block everything enqueued after it — while [Offline] is transient and the
 * item must stay queued for the next pass.
 */
sealed interface SubmitResult {
    /** The server accepted the submission (or had already applied it — idempotent replay). */
    data object Ack : SubmitResult

    /** The server permanently refused the submission; it must be dropped, never retried. */
    data class Rejected(val status: Int, val message: String?) : SubmitResult

    /** The computer was unreachable (or failing transiently); leave the item queued. */
    data object Offline : SubmitResult
}

/**
 * Outbound port for posting submissions to the computer. Implementations never throw:
 * transport and HTTP failures are mapped onto [SubmitResult] (NFR-1).
 */
interface SubmissionApi {
    suspend fun submitClaim(req: SubmitClaimReq): SubmitResult
    suspend fun requestRedemption(req: RequestRedemptionReq): SubmitResult
    suspend fun requestCashOut(req: RequestCashOutReq): SubmitResult
}

/** Ids the server now knows about, derived from a refreshed `my_claims`/`my_requests`/`my_cashouts`. */
data class ResolvedIds(val claims: Set<Long>, val requests: Set<Long>, val cashouts: Set<Long>)

/**
 * Inbound port yielding the set of submission ids the server has acknowledged,
 * used to reconcile the outbox after a flush.
 */
interface StatePort {
    suspend fun resolvedIds(): ResolvedIds
}
