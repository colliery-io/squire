package com.squire.knight.app.data

import com.squire.knight.core.KnightCommand
import com.squire.knight.core.KnightSubmit
import com.squire.knight.core.ReviewFetcher
import com.squire.knight.core.SubmitResult
import com.squire.sdk.api.ControlApi
import com.squire.sdk.api.KnightApi
import com.squire.sdk.infrastructure.ClientException
import com.squire.sdk.infrastructure.ServerException
import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.LoginReq
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Interceptor
import okhttp3.OkHttpClient
import java.io.IOException
import java.util.concurrent.atomic.AtomicReference

/**
 * Transport adapter binding `:knight-core`'s ports to the generated [KnightApi].
 *
 * It implements both outbound ports the Knight needs:
 *  - [ReviewFetcher] — `GET /household-review` (throws on failure; the store falls back to cache);
 *  - [KnightSubmit]  — the privileged quick-actions, mapping HTTP status onto [SubmitResult] so the
 *    `:knight-core` sync engine can drain idempotently and **never throws**.
 *
 * Auth mirrors the Squire adapter (T-0037): a mutable [tokenHolder] feeds an okhttp [Interceptor],
 * and [ensureLoggedIn] logs in **lazily** before any call when the token is empty — so an app that
 * started offline still authenticates the moment the Keep is reachable, rather than POSTing
 * privileged commands with no `Authorization` header. The Knight logs in as a **Knight-role** user
 * (the parent's per-user account; demo: user 1) — a Squire token would 403 every action.
 *
 * Pairing / secure token storage stay deferred (NFR-5) — [baseUrl]/[household]/[user]/[secret] come
 * from a placeholder demo config in `MainActivity`.
 */
class KnightApiAdapter(
    private val baseUrl: String,
    private val household: String,
    private val user: Long,
    private val secret: String,
) : ReviewFetcher, KnightSubmit {

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

    private val api = KnightApi(basePath = baseUrl, client = client)
    private val control = ControlApi(basePath = baseUrl, client = client)

    /** Exchange the demo `(household, user, secret)` for a Knight-scoped token; store it. */
    suspend fun login(): String = withContext(Dispatchers.IO) {
        val resp = control.login(LoginReq(household = household, secret = secret, user = user))
        tokenHolder.set(resp.token)
        resp.token
    }

    /** Lazily log in when we hold no token yet (the offline-start case; see class doc). */
    private suspend fun ensureLoggedIn() {
        if (tokenHolder.get().isEmpty()) {
            login()
        }
    }

    override suspend fun fetch(): HouseholdReview = withContext(Dispatchers.IO) {
        ensureLoggedIn()
        api.householdReview(xHousehold = household)
    }

    override suspend fun submit(command: KnightCommand): SubmitResult = withContext(Dispatchers.IO) {
        runCatching {
            ensureLoggedIn()
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
     *  - **409** (`AlreadyReviewed`) / **403** (`OccurrenceTaken`, the Race already won) → already
     *    done, nothing left to do — resolve the item;
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
