package com.squire.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.squire.core.PlayerStore
import com.squire.core.PlayerUiState
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/**
 * Thin lifecycle wrapper over the pure-Kotlin [PlayerStore]. It re-exposes the
 * store's [state] flow and launches its suspend operations in [viewModelScope];
 * all behaviour (offline fallback, outbox, sync) lives in `:core`.
 */
class SquireViewModel(private val store: PlayerStore) : ViewModel() {

    val state: StateFlow<PlayerUiState> = store.state

    /**
     * The Refresh action (and startup) does the spec's "Sync" = flush the durable outbox, then
     * refetch — so a submission queued in a previous session (survived a restart) is delivered as
     * soon as the computer is reachable, not just on the next manual submit.
     */
    fun refresh() = viewModelScope.launch { store.syncNow() }

    fun submitClaim(questId: Long, on: Int) {
        viewModelScope.launch { store.submitClaim(questId = questId, on = on) }
    }

    fun requestRedemption(itemId: Long) {
        viewModelScope.launch { store.requestRedemption(itemId = itemId) }
    }
}
