package com.squire.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.squire.app.data.RoomOutbox
import com.squire.app.data.RoomStateCache
import com.squire.app.data.SquireApiAdapter
import com.squire.app.data.db.SquireDb
import com.squire.app.ui.PlayerHomeScreen
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

    // DEMO/placeholder transport config — matches the `serve_demo` seed (SQUIRE-T-0035).
    // `10.0.2.2` is the host loopback from the emulator. Pairing UI + secure token storage
    // (real login credentials) land in a later task; for now these are baked demo creds.
    private val baseUrl = "http://10.0.2.2:8080"
    private val household = "demo"
    private val user = 2L
    private val secret = "demo"

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val adapter = SquireApiAdapter(
            baseUrl = baseUrl,
            household = household,
            user = user,
            secret = secret,
        )
        val json = Json { ignoreUnknownKeys = true }
        // Durable cache + outbox: queued claims and the cached StateView survive app/process
        // restart (REQ-SY1/SY2). Built once here and shared by the PlayerStore + SyncEngine.
        val db = SquireDb.build(this)
        val outbox = RoomOutbox(db.outboxDao(), json)
        val store = PlayerStore(
            fetcher = adapter,
            cache = RoomStateCache(db.cacheDao()),
            outbox = outbox,
            sync = SyncEngine(outbox, adapter, adapter),
            json = json,
            ids = { ids.getAndIncrement() },
        )
        val viewModel = SquireViewModel(store)

        setContent {
            MaterialTheme {
                val state by viewModel.state.collectAsStateWithLifecycle()

                // DEMO connect: log in once to obtain a tenant token, then refresh. If login
                // fails (server down / wrong creds) the PlayerStore's offline path still shows the
                // cached/empty state gracefully, so we swallow the error and refresh regardless.
                LaunchedEffect(Unit) {
                    try {
                        adapter.login()
                    } catch (_: Throwable) {
                        // Offline / login failure — fall through to refresh (offline path).
                    }
                    viewModel.refresh()
                }

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
