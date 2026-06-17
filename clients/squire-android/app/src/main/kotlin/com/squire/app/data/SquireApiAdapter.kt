package com.squire.app.data

import com.squire.core.ResolvedIds
import com.squire.core.StateFetcher
import com.squire.core.StatePort
import com.squire.core.SubmissionApi
import com.squire.sdk.api.SquireApi
import com.squire.sdk.model.RequestRedemptionReq
import com.squire.sdk.model.StateView
import com.squire.sdk.model.SubmitClaimReq
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
 * The generated [SquireApi] threads `X-Household` per call; the bearer token is added
 * by an okhttp [Interceptor] on the configured client. The blocking okhttp calls are
 * moved off the caller's thread onto [Dispatchers.IO].
 *
 * Pairing/token storage is a later task — for now [baseUrl], [household], [token] are
 * injected from a placeholder config (see `MainActivity`).
 */
class SquireApiAdapter(
    baseUrl: String,
    private val household: String,
    token: String,
) : StateFetcher, SubmissionApi, StatePort {

    private val client: OkHttpClient = OkHttpClient.Builder()
        .addInterceptor(
            Interceptor { chain ->
                val authed = chain.request().newBuilder()
                    .header("Authorization", "Bearer $token")
                    .build()
                chain.proceed(authed)
            },
        )
        .build()

    private val api = SquireApi(basePath = baseUrl, client = client)

    override suspend fun fetchState(): StateView = withContext(Dispatchers.IO) {
        api.getState(xHousehold = household)
    }

    override suspend fun submitClaim(req: SubmitClaimReq) {
        withContext(Dispatchers.IO) {
            api.submitClaim(xHousehold = household, submitClaimReq = req)
        }
    }

    override suspend fun requestRedemption(req: RequestRedemptionReq) {
        withContext(Dispatchers.IO) {
            api.requestRedemption(xHousehold = household, requestRedemptionReq = req)
        }
    }

    override suspend fun resolvedIds(): ResolvedIds = withContext(Dispatchers.IO) {
        val state = api.getState(xHousehold = household)
        ResolvedIds(
            claims = state.myClaims.map { it.claimId }.toSet(),
            requests = state.myRequests.map { it.requestId }.toSet(),
        )
    }
}
