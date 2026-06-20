package com.squire.knight.app.data

import com.squire.knight.core.KnightCommand
import com.squire.knight.core.KnightSubmit
import com.squire.knight.core.ReviewFetcher
import com.squire.knight.core.SubmitResult
import com.squire.sdk.api.ControlApi
import com.squire.sdk.api.KnightApi
import com.squire.sdk.infrastructure.ClientException
import com.squire.sdk.infrastructure.ServerException
import com.squire.sdk.model.AchievementSummaryDto
import com.squire.sdk.model.AddMemberReq
import com.squire.sdk.model.AddMemberResp
import com.squire.sdk.model.CreateAchievementReq
import com.squire.sdk.model.CreateItemReq
import com.squire.sdk.model.CreateQuestReq
import com.squire.sdk.model.CreatedAchievement
import com.squire.sdk.model.CreatedItem
import com.squire.sdk.model.CreatedQuest
import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.ItemSummaryDto
import com.squire.sdk.model.MemberSummaryDto
import com.squire.sdk.model.MintPairCodeReq
import com.squire.sdk.model.MintPairCodeResp
import com.squire.sdk.model.QuestSummaryDto
import com.squire.sdk.model.Role
import com.squire.sdk.model.SetActiveReq
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
    // Control-plane (add member, mint pairing code) — both RequireKnight, so they ride the same
    // bearer-injecting client as the Knight surface (SQUIRE-T-0075).
    private val control = ControlApi(basePath = baseUrl, client = client)

    override suspend fun fetch(): HouseholdReview = withContext(Dispatchers.IO) {
        api.householdReview(xHousehold = household)
    }

    /** The "assume Squire" read (SQUIRE-T-0053): a chosen Squire's full [StateView] (Knight-gated). */
    suspend fun squireState(squire: Long): StateView = withContext(Dispatchers.IO) {
        api.squireState(xHousehold = household, id = squire)
    }

    // ── Quest authoring from the phone (SQUIRE-T-0064/0065) ──────────────────────────────────────
    /** Every quest (active + archived) as flat summaries with server-computed labels. */
    suspend fun listQuests(): List<QuestSummaryDto> = withContext(Dispatchers.IO) {
        api.listQuests(xHousehold = household)
    }

    /** Create or edit a quest; returns the (new or kept) id. */
    suspend fun createQuest(req: CreateQuestReq): CreatedQuest = withContext(Dispatchers.IO) {
        api.createQuest(xHousehold = household, createQuestReq = req)
    }

    /** Archive a quest (never deletes). */
    suspend fun archiveQuest(id: Long) = withContext(Dispatchers.IO) {
        api.archiveQuest(xHousehold = household, id = id)
    }

    // ── Achievement authoring from the phone (SQUIRE-T-0072) ─────────────────────────────────────
    /** Every achievement as a flat summary (server-computed label). */
    suspend fun listAchievements(): List<AchievementSummaryDto> = withContext(Dispatchers.IO) {
        api.listAchievements(xHousehold = household)
    }

    /** Create or edit an achievement; returns the (new or kept) id. */
    suspend fun createAchievement(req: CreateAchievementReq): CreatedAchievement = withContext(Dispatchers.IO) {
        api.createAchievement(xHousehold = household, createAchievementReq = req)
    }

    /** Archive an achievement (never deletes). */
    suspend fun archiveAchievement(id: Long) = withContext(Dispatchers.IO) {
        api.archiveAchievement(xHousehold = household, id = id)
    }

    // ── Reward (item) authoring from the phone (SQUIRE-T-0074) ───────────────────────────────────
    /** Every reward as a flat summary (server-computed availability/gate label). */
    suspend fun listItems(): List<ItemSummaryDto> = withContext(Dispatchers.IO) {
        api.listItems(xHousehold = household)
    }

    /** Create or edit a reward; returns the (new or kept) id. */
    suspend fun createItem(req: CreateItemReq): CreatedItem = withContext(Dispatchers.IO) {
        api.createItem(xHousehold = household, createItemReq = req)
    }

    /** Archive a reward (never deletes). */
    suspend fun archiveItem(id: Long) = withContext(Dispatchers.IO) {
        api.archiveItem(xHousehold = household, id = id)
    }

    // ── Hazard catalog (SQUIRE-T-0096): shared config; applying one reuses the adjust above ───────
    /** The household's hazard catalog (named penalties). */
    suspend fun listHazards(): List<com.squire.sdk.model.Hazard> = withContext(Dispatchers.IO) {
        api.listHazards(xHousehold = household)
    }

    /** Replace the household's hazard catalog. */
    suspend fun setHazards(hazards: List<com.squire.sdk.model.Hazard>) = withContext(Dispatchers.IO) {
        api.setHazards(xHousehold = household, hazard = hazards)
    }

    // ── Member administration from the phone (SQUIRE-T-0075) ─────────────────────────────────────
    /** Every household member (Knights + Squires) with role + active flag. */
    suspend fun listMembers(): List<MemberSummaryDto> = withContext(Dispatchers.IO) {
        api.listMembers(xHousehold = household)
    }

    /** Add a Knight or Squire (control-plane, Knight-gated); returns the new member's id. */
    suspend fun addMember(displayName: String, role: Role, secret: String): AddMemberResp = withContext(Dispatchers.IO) {
        control.addMember(xHousehold = household, addMemberReq = AddMemberReq(displayName = displayName, initialSecret = secret, role = role))
    }

    /** De/reactivate a member (archive-not-delete). */
    suspend fun setMemberActive(id: Long, active: Boolean) = withContext(Dispatchers.IO) {
        api.setMemberActive(xHousehold = household, id = id, setActiveReq = SetActiveReq(active = active))
    }

    /** Mint a one-time pairing code for a member (control-plane, Knight-gated). */
    suspend fun mintPairCode(user: Long): MintPairCodeResp = withContext(Dispatchers.IO) {
        control.mintPairCode(xHousehold = household, mintPairCodeReq = MintPairCodeReq(user = user))
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
