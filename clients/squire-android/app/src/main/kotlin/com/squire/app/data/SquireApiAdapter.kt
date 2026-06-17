package com.squire.app.data

import com.squire.core.ResolvedIds
import com.squire.core.StateFetcher
import com.squire.core.StatePort
import com.squire.core.SubmissionApi
import com.squire.sdk.api.ControlApi
import com.squire.sdk.api.SquireApi
import com.squire.sdk.model.LoginReq
import com.squire.sdk.model.RequestRedemptionReq
import com.squire.sdk.model.StateView
import com.squire.sdk.model.SubmitClaimReq
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Interceptor
import okhttp3.OkHttpClient
import java.util.concurrent.atomic.AtomicReference

/**
 * Transport adapter binding `:core`'s ports to the generated okhttp [SquireApi].
 *
 * One instance implements all three outbound ports the presentation layer needs:
 *  - [StateFetcher]  — `GET /state`
 *  - [SubmissionApi] — `POST /claims`, `POST /redemption-requests`
 *  - [StatePort]     — reconciliation ids derived from the refreshed `my_claims`/`my_requests`.
 *
 * The generated [SquireApi] threads `X-Household` per call; the bearer token is added by an
 * okhttp [Interceptor] that reads a **mutable** token holder ([tokenHolder]). The token starts
 * empty; [login] exchanges `(household, user, secret)` for a tenant-scoped token via the
 * control-plane [ControlApi] and stores it in the holder, so every subsequent [SquireApi] call
 * carries `Authorization: Bearer <token>`. Building the holder up-front (rather than baking the
 * token into the client) lets the same client be used before and after login. The blocking okhttp
 * calls are moved off the caller's thread onto [Dispatchers.IO].
 *
 * Pairing / secure token storage is a later task — for now [baseUrl], [household], [user], and
 * [secret] are injected from a placeholder demo config (see `MainActivity`).
 */
class SquireApiAdapter(
    private val baseUrl: String,
    private val household: String,
    private val user: Long,
    private val secret: String,
) : StateFetcher, SubmissionApi, StatePort {

    /** Mutable bearer token, set after [login]. Empty until then. */
    private val tokenHolder = AtomicReference("")

    private val client: OkHttpClient = OkHttpClient.Builder()
        .addInterceptor(
            Interceptor { chain ->
                val token = tokenHolder.get()
                val builder = chain.request().newBuilder()
                if (token.isNotEmpty()) {
                    builder.header("Authorization", "Bearer $token")
                }
                chain.proceed(builder.build())
            },
        )
        .build()

    private val api = SquireApi(basePath = baseUrl, client = client)
    private val control = ControlApi(basePath = baseUrl, client = client)

    /**
     * Exchange the demo `(household, user, secret)` for a tenant-scoped token via `POST /login`,
     * store it in [tokenHolder] (so subsequent [SquireApi] calls are authenticated), and return it.
     */
    suspend fun login(): String = withContext(Dispatchers.IO) {
        val resp = control.login(LoginReq(household = household, secret = secret, user = user))
        tokenHolder.set(resp.token)
        resp.token
    }

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
