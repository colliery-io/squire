package com.squire.knight.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.squire.knight.core.KnightStore
import com.squire.knight.core.KnightUiState
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/**
 * Thin lifecycle wrapper over the pure-Kotlin [KnightStore]. It re-exposes the store's [state] flow
 * and launches its suspend operations in [viewModelScope]; all behaviour (offline fallback, the
 * privileged outbox, ack-based sync) lives in `:knight-core`.
 */
class KnightViewModel(private val store: KnightStore) : ViewModel() {

    val state: StateFlow<KnightUiState> = store.state

    /** Refresh (and startup) = the spec's "Sync": drain the durable outbox, then refetch (REQ-K9). */
    fun refresh() = viewModelScope.launch { store.syncNow() }

    fun approveClaim(claimId: Long) = viewModelScope.launch { store.approveClaim(claimId) }
    fun rejectClaim(claimId: Long, reason: String?) =
        viewModelScope.launch { store.rejectClaim(claimId, reason) }

    fun approveRequest(requestId: Long) = viewModelScope.launch { store.approveRequest(requestId) }
    fun rejectRequest(requestId: Long, reason: String?) =
        viewModelScope.launch { store.rejectRequest(requestId, reason) }

    fun redeem(squire: Long, itemId: Long) = viewModelScope.launch { store.redeem(squire, itemId) }
    fun adjust(squire: Long, amount: Long, reason: String) =
        viewModelScope.launch { store.adjust(squire, amount, reason) }

    /** Settle real-money cash owed to [squire] (SQUIRE-T-0111): a Cash payout of −[amount]. */
    fun pay(squire: Long, amount: Long, reason: String) =
        viewModelScope.launch { store.pay(squire, amount, reason) }

    fun markDone(squire: Long, questId: Long, on: Int) =
        viewModelScope.launch { store.markDone(squire, questId, on) }
}
