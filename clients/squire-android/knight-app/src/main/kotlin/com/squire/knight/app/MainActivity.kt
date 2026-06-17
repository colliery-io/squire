package com.squire.knight.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import com.squire.knight.BuildConfig
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.knight.app.data.RoomPrivilegedOutbox
import com.squire.knight.app.data.RoomReviewCache
import com.squire.knight.app.data.db.KnightDb
import com.squire.knight.app.ui.KnightHomeScreen
import com.squire.knight.core.KnightStore
import com.squire.knight.core.KnightSyncEngine
import com.squire.pairing.NsdDiscovery
import com.squire.pairing.PairingScreen
import com.squire.pairing.Session
import com.squire.pairing.SessionStore
import com.squire.sdk.api.ControlApi
import com.squire.sdk.model.LoginReq
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import java.util.concurrent.atomic.AtomicLong

/** Foreground auto-refresh cadence (SQUIRE-T-0041). */
private const val AUTO_REFRESH_MS = 5_000L

/**
 * Compose host for the Knight (parent) review home.
 *
 * **Session-gated** (ADR SQUIRE-A-0010 / SQUIRE-T-0046): first run shows the pairing screen; once a
 * [Session] is stored (Keystore-encrypted) it builds the transport from that session's host +
 * Knight-role token and shows the review home. "Forget" clears the session. A debug-only demo
 * bypass logs in as the Knight (user 1) so the emulator flow stays one-tap; release always pairs.
 */
class MainActivity : ComponentActivity() {

    private val ids = AtomicLong(System.currentTimeMillis())

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val sessionStore = SessionStore(this)
        val discovery = NsdDiscovery(this)
        val db = KnightDb.build(this)
        val json = Json { ignoreUnknownKeys = true }

        setContent {
            MaterialTheme {
                var session by remember { mutableStateOf(sessionStore.load()) }
                val current = session
                if (current == null) {
                    PairingScreen(
                        discovery = discovery,
                        demoLogin = if (BuildConfig.DEBUG) ({ demoLogin() }) else null,
                        onPaired = { s -> sessionStore.save(s); session = s },
                    )
                } else {
                    KnightHomeHost(
                        session = current,
                        db = db,
                        json = json,
                        ids = ids,
                        onForget = {
                            sessionStore.clear()
                            session = null
                        },
                    )
                }
            }
        }
    }

    /** Debug-only bypass: log in with the demo Knight creds against `squire-home` → a [Session]. */
    private suspend fun demoLogin(): Session = withContext(Dispatchers.IO) {
        val control = ControlApi(basePath = "http://10.0.2.2:8080")
        val resp = control.login(LoginReq(household = "demo", secret = "demo", user = 1L))
        Session(
            host = "10.0.2.2",
            port = 8080,
            household = "demo",
            token = resp.token,
            user = 1L,
            role = resp.role.value,
        )
    }
}

/** Builds the offline-first [KnightStore] from a paired [Session] and hosts the review home. */
@Composable
private fun KnightHomeHost(
    session: Session,
    db: KnightDb,
    json: Json,
    ids: AtomicLong,
    onForget: () -> Unit,
) {
    val viewModel = remember(session) {
        val adapter = KnightApiAdapter(session.baseUrl, session.household, session.token)
        val outbox = RoomPrivilegedOutbox(db.outboxDao(), json)
        val store = KnightStore(
            fetcher = adapter,
            cache = RoomReviewCache(db.reviewCacheDao()),
            outbox = outbox,
            sync = KnightSyncEngine(outbox, adapter),
            json = json,
            ids = { ids.getAndIncrement() },
        )
        KnightViewModel(store)
    }

    val state by viewModel.state.collectAsStateWithLifecycle()
    val lifecycle = LocalLifecycleOwner.current.lifecycle

    LaunchedEffect(session) {
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
        onForget = onForget,
    )
}
