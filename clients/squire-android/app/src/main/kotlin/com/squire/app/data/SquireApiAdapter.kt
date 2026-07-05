package com.squire.app.data

import com.squire.core.ResolvedIds
import com.squire.core.StateFetcher
import com.squire.core.StatePort
import com.squire.core.SubmissionApi
import com.squire.core.SubmitResult
import com.squire.sdk.api.SquireApi
import com.squire.sdk.infrastructure.ClientException
import com.squire.sdk.infrastructure.ServerException
import com.squire.sdk.model.RequestCashOutReq
import com.squire.sdk.model.RequestRedemptionReq
import com.squire.sdk.model.StateView
import com.squire.sdk.model.SubmitClaimReq
import java.io.IOException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Interceptor
import okhttp3.OkHttpClient

/**
 * Transport adapter binding `:core`'s ports to the generated okhttp [SquireApi].
 *
 * One instance implements all three outbound ports the presentation layer needs:
 *  - [StateFetcher]  — `GET /state`
 *  - [SubmissionApi] — `POST /claims`, `POST /redemption-requests`
 *  - [StatePort]     — reconciliation ids derived from the refreshed `my_claims`/`my_requests`.
 *
 * Auth comes from a **paired session** (ADR SQUIRE-A-0010): [baseUrl], [household], and the
 * per-user [token] are obtained once via device pairing (`POST /pair`) and stored encrypted, then
 * injected here. An okhttp [Interceptor] attaches `Authorization: Bearer <token>` to every call —
 * no in-app login, no baked credentials. (A token that has expired surfaces as 401s; the host app
 * can "forget device" to re-pair.) The blocking okhttp calls run on [Dispatchers.IO].
 */
class SquireApiAdapter(
    private val baseUrl: String,
    private val household: String,
    private val token: String,
) : StateFetcher, SubmissionApi, StatePort {

    private val client: OkHttpClient = OkHttpClient.Builder()
        .addInterceptor(
            Interceptor { chain ->
                chain.proceed(
                    chain.request().newBuilder().header("Authorization", "Bearer $token").build(),
                )
            },
        )
        .build()

    private val api = SquireApi(basePath = baseUrl, client = client)

    override suspend fun fetchState(): StateView = withContext(Dispatchers.IO) {
        api.getState(xHousehold = household)
    }

    override suspend fun submitClaim(req: SubmitClaimReq): SubmitResult = submit {
        api.submitClaim(xHousehold = household, submitClaimReq = req)
    }

    override suspend fun requestRedemption(req: RequestRedemptionReq): SubmitResult = submit {
        api.requestRedemption(xHousehold = household, requestRedemptionReq = req)
    }

    override suspend fun requestCashOut(req: RequestCashOutReq): SubmitResult = submit {
        api.requestCashout(xHousehold = household, requestCashOutReq = req)
    }

    private suspend fun submit(call: () -> Any): SubmitResult = withContext(Dispatchers.IO) {
        runCatching { call() }.fold(
            onSuccess = { SubmitResult.Ack },
            onFailure = { e -> e.toSubmitResult() },
        )
    }

    /**
     * Map a thrown transport error onto a [SubmitResult] (same convention as `KnightApiAdapter`):
     *  - 401/403 → the *session* is broken (expired/revoked token), not the submission — keep the
     *    item queued; it becomes sendable again once the device re-pairs;
     *  - other 4xx (409 already-claimed, 400, 404, 422) → terminal, drop so it can't wedge the
     *    queue and starve later submissions (SQUIRE-T-0128);
     *  - 5xx and any IO / unknown error → treat as offline (transient) and keep the item.
     */
    private fun Throwable.toSubmitResult(): SubmitResult = when (this) {
        is ClientException -> when (statusCode) {
            401, 403 -> SubmitResult.Offline
            in 400..499 -> SubmitResult.Rejected(statusCode, message)
            else -> SubmitResult.Offline
        }
        is ServerException -> SubmitResult.Offline
        is IOException -> SubmitResult.Offline
        else -> SubmitResult.Offline
    }

    override suspend fun resolvedIds(): ResolvedIds = withContext(Dispatchers.IO) {
        val state = api.getState(xHousehold = household)
        ResolvedIds(
            claims = state.myClaims.map { it.claimId }.toSet(),
            requests = state.myRequests.map { it.requestId }.toSet(),
            cashouts = state.myCashouts.orEmpty().map { it.requestId }.toSet(),
        )
    }
}
