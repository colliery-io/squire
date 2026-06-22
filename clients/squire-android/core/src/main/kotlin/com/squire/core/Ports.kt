package com.squire.core

import com.squire.sdk.model.RequestCashOutReq
import com.squire.sdk.model.RequestRedemptionReq
import com.squire.sdk.model.SubmitClaimReq

/**
 * Outbound port for posting submissions to the computer. Implementations throw
 * on network failure; the [SyncEngine] treats any throw as "computer unreachable".
 */
interface SubmissionApi {
    suspend fun submitClaim(req: SubmitClaimReq)
    suspend fun requestRedemption(req: RequestRedemptionReq)
    suspend fun requestCashOut(req: RequestCashOutReq)
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
