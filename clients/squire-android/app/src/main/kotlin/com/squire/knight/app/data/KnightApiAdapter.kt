package com.squire.knight.app.data

import com.squire.knight.core.KnightCommand
import com.squire.knight.core.KnightSubmit
import com.squire.knight.core.ReviewFetcher
import com.squire.knight.core.SubmitResult
import com.squire.sdk.api.KnightApi
import com.squire.sdk.infrastructure.ClientException
import com.squire.sdk.infrastructure.ServerException
import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.StateView
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Interceptor
import okhttp3.OkHttpClient
import java.io.IOException

/**
 * Transport adapter binding `:knight-core`'s ports to the generated [KnightApi].
 *
 * It implements both outbound ports the Knight needs:
 *  - [ReviewFetcher] — `GET /household-review` (throws on failure; the store falls back to cache);
 *  - [KnightSubmit]  — the privileged quick-actions, mapping HTTP status onto [SubmitResult] so the
 *    `:knight-core` sync engine can drain idempotently and **never throws**.
 *
 * Auth comes from a **paired session** (ADR SQUIRE-A-0010): [baseUrl], [household], and the
 * Knight-role [token] are obtained once via device pairing (`POST /pair`) and stored encrypted, then
 * injected here — no in-app login, no baked credentials. An okhttp [Interceptor] attaches the bearer
 * token to every call.
 */
class KnightApiAdapter(
    private val baseUrl: String,
    private val household: String,
    private val token: String,
) : ReviewFetcher, KnightSubmit {

    private val client: OkHttpClient = OkHttpClient.Builder()
        .addInterceptor(
            Interceptor { chain ->
                chain.proceed(
                    chain.request().newBuilder().header("Authorization", "Bearer $token").build(),
                )
            },
        )
        .build()

    private val api = KnightApi(basePath = baseUrl, client = client)

    override suspend fun fetch(): HouseholdReview = withContext(Dispatchers.IO) {
        api.householdReview(xHousehold = household)
    }

    /** The "assume Squire" read (SQUIRE-T-0053): a chosen Squire's full [StateView] (Knight-gated). */
    suspend fun squireState(squire: Long): StateView = withContext(Dispatchers.IO) {
        api.squireState(xHousehold = household, id = squire)
    }

    override suspend fun submit(command: KnightCommand): SubmitResult = withContext(Dispatchers.IO) {
        runCatching {
            when (command) {
                is KnightCommand.ReviewClaim -> api.reviewClaim(household, command.req)
                is KnightCommand.ReviewRedemption -> api.reviewRedemption(household, command.req)
                is KnightCommand.Redeem -> api.redeem(household, command.req)
                is KnightCommand.Adjust -> api.adjust(household, command.req)
                is KnightCommand.MarkDone -> api.markDone(household, command.req)
            }
        }.fold(
            onSuccess = { SubmitResult.Ack },
            onFailure = { e -> e.toSubmitResult() },
        )
    }

    /**
     * Map a thrown transport error onto a [SubmitResult]:
     *  - 409 (`AlreadyReviewed`) / 403 (`OccurrenceTaken`, the Race already won) → already done;
     *  - other 4xx (400 blank reason, 404 not found, 422) → poison, drop so it can't loop;
     *  - 5xx and any IO / unknown error → treat as offline (transient) and keep the item.
     */
    private fun Throwable.toSubmitResult(): SubmitResult = when (this) {
        is ClientException -> when (statusCode) {
            409, 403 -> SubmitResult.AlreadyDone
            in 400..499 -> SubmitResult.Poison(statusCode, message)
            else -> SubmitResult.Offline
        }
        is ServerException -> SubmitResult.Offline
        is IOException -> SubmitResult.Offline
        else -> SubmitResult.Offline
    }
}
