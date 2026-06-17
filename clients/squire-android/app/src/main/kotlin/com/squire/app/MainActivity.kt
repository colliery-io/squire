package com.squire.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.squire.app.data.SquireApiAdapter
import com.squire.app.ui.PlayerHomeScreen
import com.squire.core.InMemoryOutbox
import com.squire.core.InMemoryStateCache
import com.squire.core.PlayerStore
import com.squire.core.PlayerUiState
import com.squire.core.SyncEngine
import kotlinx.serialization.json.Json
import java.util.concurrent.atomic.AtomicLong

/**
 * Compose host for the Squire player home.
 *
 * Wiring only: it assembles a [PlayerStore] over the in-memory `:core` impls and a
 * [SquireApiAdapter] built from a placeholder transport config. Durable cache/outbox,
 * pairing, and real token storage are deferred to follow-up tasks (see SQUIRE-T-0034).
 */
class MainActivity : ComponentActivity() {

    private val ids = AtomicLong(System.currentTimeMillis())

    // Placeholder transport config. `10.0.2.2` is the host loopback from the emulator.
    // Pairing UI + secure token storage land in a later task.
    private val baseUrl = "http://10.0.2.2:8080"
    private val household = "dev-household"
    private val token = "dev-token"

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val adapter = SquireApiAdapter(baseUrl = baseUrl, household = household, token = token)
        val outbox = InMemoryOutbox()
        val json = Json { ignoreUnknownKeys = true }
        val store = PlayerStore(
            fetcher = adapter,
            cache = InMemoryStateCache(),
            outbox = outbox,
            sync = SyncEngine(outbox, adapter, adapter),
            json = json,
            ids = { ids.getAndIncrement() },
        )
        val viewModel = SquireViewModel(store)

        setContent {
            MaterialTheme {
                val state by viewModel.state.collectAsStateWithLifecycle()

                LaunchedEffect(Unit) { viewModel.refresh() }

                PlayerHomeScreen(
                    state = state,
                    onRefresh = viewModel::refresh,
                    onMarkDone = { questId ->
                        val on = (state as? PlayerUiState.Ready)
                            ?.view
                            ?.questsToday
                            ?.firstOrNull { it.questId == questId }
                            ?.on
                            ?: 0
                        viewModel.submitClaim(questId = questId, on = on)
                    },
                    onRedeem = viewModel::requestRedemption,
                )
            }
        }
    }
}
