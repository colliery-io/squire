package com.squire.core

import com.squire.sdk.model.RequestCashOutReq
import com.squire.sdk.model.RequestRedemptionReq
import com.squire.sdk.model.StateView
import com.squire.sdk.model.SubmitClaimReq
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.json.Json

/**
 * Inbound port that fetches the authenticated Squire's full [StateView] snapshot
 * (`GET /state`). Implementations throw on network failure; [PlayerStore] catches
 * that and falls back to the cached view (NFR-1/6).
 */
interface StateFetcher {
    suspend fun fetchState(): StateView
}

/**
 * The observable presentation state of the player home.
 *
 * [Ready.fromCache] is `true` when the view was served from the offline cache
 * because the computer was unreachable — the UI surfaces this as an offline banner.
 */
sealed interface PlayerUiState {
    data object Loading : PlayerUiState
    data class Ready(val view: StateView, val fromCache: Boolean) : PlayerUiState
    data class Error(val message: String) : PlayerUiState
}

/**
 * Pure-Kotlin presentation store for the Squire (child) player home.
 *
 * It owns the offline-first read path ([refresh]) and the optimistic write path
 * ([submitClaim] / [requestRedemption]): each write mints a stable phone-side id,
 * enqueues it on the idempotent [outbox], runs a [sync] pass to flush, then refreshes.
 *
 * Nothing here ever throws: an unreachable computer degrades to the cached view
 * (NFR-1/6). The whole thing is JVM-testable with fakes — `:app` only wires transport.
 *
 * @param clock supplies "now" (unused placeholder for future TTL logic; kept for wiring parity).
 * @param ids mints the phone-side idempotency key for each new submission.
 */
class PlayerStore(
    private val fetcher: StateFetcher,
    private val cache: StateCache,
    private val outbox: Outbox,
    private val sync: SyncEngine,
    private val json: Json,
    private val clock: () -> Long = { 0L },
    private val ids: () -> Long,
) {
    private val _state = MutableStateFlow<PlayerUiState>(PlayerUiState.Loading)
    val state: StateFlow<PlayerUiState> = _state.asStateFlow()

    /**
     * Fetch the freshest [StateView] and cache it; on failure fall back to the last
     * cached view flagged [PlayerUiState.Ready.fromCache]. Never throws.
     */
    suspend fun refresh() {
        try {
            val view = fetcher.fetchState()
            cache.save(json.encodeToString(StateView.serializer(), view))
            _state.value = PlayerUiState.Ready(view, fromCache = false)
        } catch (e: Exception) {
            val raw = cache.load()
            if (raw != null) {
                try {
                    val cached = json.decodeFromString(StateView.serializer(), raw)
                    _state.value = PlayerUiState.Ready(cached, fromCache = true)
                } catch (decodeError: Exception) {
                    _state.value =
                        PlayerUiState.Error("Computer unreachable and cached state is unreadable.")
                }
            } else {
                _state.value =
                    PlayerUiState.Error("Computer unreachable and no cached state available.")
            }
        }
    }

    /**
     * The spec's "Sync" (REQ-SY4): flush the durable [outbox] (post each pending submission —
     * idempotent on its phone-minted id), then refetch. Use on app startup and for the user's
     * Refresh action so a submission queued in a PREVIOUS session (one that survived an app
     * restart via the durable outbox) is delivered as soon as the computer is reachable. Never throws.
     */
    suspend fun syncNow() {
        sync.sync()
        refresh()
    }

    /**
     * Submit a completion claim for the quest [questId] on day [on]. Mints an id,
     * enqueues the claim, flushes via [sync], then refreshes. Never throws.
     */
    suspend fun submitClaim(questId: Long, on: Int) {
        val id = ids()
        outbox.enqueue(OutboxItem.Claim(SubmitClaimReq(claimId = id, questId = questId, on = on)))
        sync.sync()
        refresh()
    }

    /**
     * Request redemption of reward [itemId]. Mints an id, enqueues the request,
     * flushes via [sync], then refreshes. Never throws.
     */
    suspend fun requestRedemption(itemId: Long) {
        val id = ids()
        outbox.enqueue(
            OutboxItem.Redemption(RequestRedemptionReq(itemId = itemId, requestId = id)),
        )
        sync.sync()
        refresh()
    }

    /** Request to "cash out" [amount] whole dollars of owed Cash (SQUIRE-T-0118) — parent-approved. */
    suspend fun requestCashOut(amount: Long) {
        val id = ids()
        outbox.enqueue(OutboxItem.CashOut(RequestCashOutReq(amount = amount, requestId = id)))
        sync.sync()
        refresh()
    }
}
