package com.squire.knight.core

import com.squire.sdk.model.AdjustReq
import com.squire.sdk.model.DecisionDto
import com.squire.sdk.model.DecisionKind
import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.MarkDoneReq
import com.squire.sdk.model.RedeemReq
import com.squire.sdk.model.ReviewClaimReq
import com.squire.sdk.model.ReviewRedemptionReq
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.json.Json

/**
 * The observable presentation state of the Knight review home.
 *
 * [Ready.fromCache] is `true` when the view was served from the offline cache because the Keep was
 * unreachable — the UI surfaces this as an offline banner (REQ-K10).
 */
sealed interface KnightUiState {
    data object Loading : KnightUiState
    data class Ready(val review: HouseholdReview, val fromCache: Boolean) : KnightUiState
    data class Error(val message: String) : KnightUiState
}

/**
 * Pure-Kotlin presentation store for the Knight (parent) review home — the privileged sibling of
 * the Squire `PlayerStore`.
 *
 * Read path ([refresh]) is offline-first: fetch the cross-Squire [HouseholdReview], cache it, and on
 * failure fall back to the last cached view flagged [KnightUiState.Ready.fromCache]. Write path (the
 * quick-actions) is optimistic + durable: each action wraps an `:sdk` request DTO into a
 * [KnightCommand] (minting a `command_id`/`claim_id` where there is no natural key), enqueues it on
 * the idempotent [outbox], drains via [sync], then refreshes.
 *
 * Nothing here ever throws — an unreachable Keep degrades to the cached queue (NFR-1). JVM-testable
 * with fakes; `:knight-app` only wires transport + Room.
 *
 * @param ids mints the phone-side idempotency key for direct redeem / adjust / mark-done.
 */
class KnightStore(
    private val fetcher: ReviewFetcher,
    private val cache: ReviewCache,
    private val outbox: PrivilegedOutbox,
    private val sync: KnightSyncEngine,
    private val json: Json,
    private val ids: () -> Long,
) {
    private val _state = MutableStateFlow<KnightUiState>(KnightUiState.Loading)
    val state: StateFlow<KnightUiState> = _state.asStateFlow()

    /**
     * Fetch the freshest [HouseholdReview] and cache it; on failure fall back to the last cached
     * view flagged [KnightUiState.Ready.fromCache]. Never throws.
     */
    suspend fun refresh() {
        try {
            val review = fetcher.fetch()
            cache.save(json.encodeToString(HouseholdReview.serializer(), review))
            _state.value = KnightUiState.Ready(review, fromCache = false)
        } catch (e: Exception) {
            val raw = cache.load()
            if (raw != null) {
                try {
                    val cached = json.decodeFromString(HouseholdReview.serializer(), raw)
                    _state.value = KnightUiState.Ready(cached, fromCache = true)
                } catch (decodeError: Exception) {
                    _state.value =
                        KnightUiState.Error("Computer unreachable and cached state is unreadable.")
                }
            } else {
                _state.value =
                    KnightUiState.Error("Computer unreachable and no cached state available.")
            }
        }
    }

    /**
     * The Knight's "Sync" (REQ-K9): drain the durable [outbox] (each privileged command, idempotent
     * on its client-minted id), then refetch. Use on app startup and the Refresh action so a command
     * queued in a PREVIOUS session (survived a restart) is delivered as soon as the Keep is
     * reachable. Never throws.
     */
    suspend fun syncNow() {
        sync.sync()
        refresh()
    }

    /** Approve a pending claim (the Squire is derived server-side from the claim). */
    suspend fun approveClaim(claimId: Long) =
        submit(KnightCommand.ReviewClaim(ReviewClaimReq(claimId = claimId, decision = DecisionDto(DecisionKind.approve))))

    /** Reject a pending claim, with an optional reason. */
    suspend fun rejectClaim(claimId: Long, reason: String? = null) =
        submit(KnightCommand.ReviewClaim(ReviewClaimReq(claimId = claimId, decision = DecisionDto(DecisionKind.reject, reason))))

    /** Approve a pending redemption request (→ ItemRedeemed). */
    suspend fun approveRequest(requestId: Long) =
        submit(KnightCommand.ReviewRedemption(ReviewRedemptionReq(decision = DecisionDto(DecisionKind.approve), requestId = requestId)))

    /** Reject a pending redemption request, with an optional reason. */
    suspend fun rejectRequest(requestId: Long, reason: String? = null) =
        submit(KnightCommand.ReviewRedemption(ReviewRedemptionReq(decision = DecisionDto(DecisionKind.reject, reason), requestId = requestId)))

    /** Direct redeem of [itemId] on [squire]'s behalf; mints a `command_id`. */
    suspend fun redeem(squire: Long, itemId: Long) =
        submit(KnightCommand.Redeem(RedeemReq(commandId = ids(), itemId = itemId, squire = squire)))

    /**
     * Add funds: a positive [amount] with a **required** [reason] (REQ-K6). The caller (UI) blocks
     * the action until a non-empty reason is supplied; this also guards defensively.
     */
    suspend fun adjust(squire: Long, amount: Long, reason: String) {
        require(reason.isNotBlank()) { "AdjustPoints requires a reason" }
        submit(KnightCommand.Adjust(AdjustReq(amount = amount, commandId = ids(), reason = reason, squire = squire)))
    }

    /** Mark quest [questId] done for [squire] on day [on] (submit-then-approve); mints a `claim_id`. */
    suspend fun markDone(squire: Long, questId: Long, on: Int) =
        submit(KnightCommand.MarkDone(MarkDoneReq(claimId = ids(), on = on, questId = questId, squire = squire)))

    /** Enqueue (idempotent), drain, then refresh — the shared optimistic write path. Never throws. */
    private suspend fun submit(command: KnightCommand) {
        outbox.enqueue(command)
        sync.sync()
        refresh()
    }
}
