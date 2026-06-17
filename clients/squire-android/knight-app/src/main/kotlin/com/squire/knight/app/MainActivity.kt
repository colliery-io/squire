package com.squire.knight.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.knight.app.data.RoomPrivilegedOutbox
import com.squire.knight.app.data.RoomReviewCache
import com.squire.knight.app.data.db.KnightDb
import com.squire.knight.app.ui.KnightHomeScreen
import com.squire.knight.core.KnightStore
import com.squire.knight.core.KnightSyncEngine
import kotlinx.coroutines.delay
import kotlinx.serialization.json.Json
import java.util.concurrent.atomic.AtomicLong

/** Foreground auto-refresh cadence (SQUIRE-T-0041). */
private const val AUTO_REFRESH_MS = 5_000L

/**
 * Compose host for the Knight (parent) review home.
 *
 * Wiring only: it assembles a [KnightStore] over the durable Room impls and a [KnightApiAdapter]
 * built from a placeholder demo config. All behaviour lives in `:knight-core`. Durable cache/outbox
 * survive restart (REQ-K7); pairing + secure token storage are deferred (NFR-5).
 */
class MainActivity : ComponentActivity() {

    private val ids = AtomicLong(System.currentTimeMillis())

    // DEMO/placeholder transport config — matches the `squire-home` seed. `10.0.2.2` is the host
    // loopback from the emulator. The Knight authenticates as a **Knight-role** user (user 1); a
    // Squire token would 403 every privileged action.
    private val baseUrl = "http://10.0.2.2:8080"
    private val household = "demo"
    private val user = 1L
    private val secret = "demo"

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val adapter = KnightApiAdapter(
            baseUrl = baseUrl,
            household = household,
            user = user,
            secret = secret,
        )
        val json = Json { ignoreUnknownKeys = true }
        val db = KnightDb.build(this)
        val outbox = RoomPrivilegedOutbox(db.outboxDao(), json)
        val store = KnightStore(
            fetcher = adapter,
            cache = RoomReviewCache(db.reviewCacheDao()),
            outbox = outbox,
            sync = KnightSyncEngine(outbox, adapter),
            json = json,
            ids = { ids.getAndIncrement() },
        )
        val viewModel = KnightViewModel(store)

        setContent {
            MaterialTheme {
                val state by viewModel.state.collectAsStateWithLifecycle()
                val lifecycle = LocalLifecycleOwner.current.lifecycle

                // DEMO connect: log in once to obtain a Knight token, then auto-refresh. If login
                // fails (server down / offline) the store's offline path renders cache gracefully,
                // and the adapter's lazy login retries on the next poll/action.
                LaunchedEffect(Unit) {
                    try {
                        adapter.login()
                    } catch (_: Throwable) {
                        // Offline / login failure — fall through to the poll loop (offline path).
                    }
                    // Foreground auto-refresh (SQUIRE-T-0041): while the review screen is visible,
                    // sync on a cadence so a child's new claim/request appears and queued approvals
                    // flush without a manual Refresh. Cancels when backgrounded; `syncNow()` never
                    // throws, so polling while the Keep is asleep is a harmless no-op.
                    lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
                        while (true) {
                            viewModel.refresh().join()
                            delay(AUTO_REFRESH_MS)
                        }
                    }
                }

                KnightHomeScreen(
                    state = state,
                    onRefresh = { viewModel.refresh() },
                    onApproveClaim = { viewModel.approveClaim(it) },
                    onRejectClaim = { viewModel.rejectClaim(it, null) },
                    onApproveRequest = { viewModel.approveRequest(it) },
                    onRejectRequest = { viewModel.rejectRequest(it, null) },
                    onAddFunds = { squire, amount, reason -> viewModel.adjust(squire, amount, reason) },
                    onRedeem = { squire, itemId -> viewModel.redeem(squire, itemId) },
                    onMarkDone = { squire, questId, on -> viewModel.markDone(squire, questId, on) },
                )
            }
        }
    }
}
