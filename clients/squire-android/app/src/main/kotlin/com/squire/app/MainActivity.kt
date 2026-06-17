package com.squire.app

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import com.squire.app.data.RoomOutbox
import com.squire.app.data.RoomStateCache
import com.squire.app.data.SquireApiAdapter
import com.squire.app.data.db.SquireDb
import com.squire.app.ui.PlayerHomeScreen
import com.squire.core.PlayerStore
import com.squire.core.PlayerUiState
import com.squire.core.SyncEngine
import com.squire.pairing.NsdDiscovery
import com.squire.pairing.PairingScreen
import com.squire.pairing.Session
import com.squire.pairing.SessionStore
import com.squire.pairing.UpdateBanner
import com.squire.pairing.UpdateChecker
import com.squire.pairing.UpdateInfo
import com.squire.sdk.api.ControlApi
import com.squire.sdk.model.LoginReq
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import java.util.concurrent.atomic.AtomicLong

/** Foreground auto-refresh cadence (SQUIRE-T-0041). */
private const val AUTO_REFRESH_MS = 5_000L

/** How often to attempt mDNS re-discovery while the server is unreachable (SQUIRE-T-0052). */
private const val RELOCATE_INTERVAL_MS = 15_000L

/**
 * Compose host for the Squire player home.
 *
 * The app is **session-gated** (ADR SQUIRE-A-0010 / SQUIRE-T-0046): on first run it shows the
 * pairing screen; once a [Session] is stored (Keystore-encrypted) it builds the transport from that
 * session's host + per-user token and shows the player home. "Forget" clears the session and
 * returns to pairing. A debug-only "Use demo creds" bypass keeps the emulator/`squire-home` flow
 * one-tap; release builds always pair.
 */
class MainActivity : ComponentActivity() {

    private val ids = AtomicLong(System.currentTimeMillis())

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val sessionStore = SessionStore(this)
        val discovery = NsdDiscovery(this)
        val db = SquireDb.build(this)
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
                    PlayerHomeHost(
                        session = current,
                        db = db,
                        json = json,
                        ids = ids,
                        discovery = discovery,
                        onSessionChanged = { s -> sessionStore.save(s); session = s },
                        onForget = {
                            sessionStore.clear()
                            session = null
                        },
                    )
                }
            }
        }
    }

    /** Debug-only bypass: log in with the demo creds against `squire-home` → a ready [Session]. */
    private suspend fun demoLogin(): Session = withContext(Dispatchers.IO) {
        val control = ControlApi(basePath = "http://10.0.2.2:8080")
        val resp = control.login(LoginReq(household = "demo", secret = "demo", user = 2L))
        Session(
            host = "10.0.2.2",
            port = 8080,
            household = "demo",
            token = resp.token,
            user = 2L,
            role = resp.role.value,
        )
    }
}

/** Builds the offline-first [PlayerStore] from a paired [Session] and hosts the player home. */
@Composable
private fun PlayerHomeHost(
    session: Session,
    db: SquireDb,
    json: Json,
    ids: AtomicLong,
    discovery: NsdDiscovery,
    onSessionChanged: (Session) -> Unit,
    onForget: () -> Unit,
) {
    val viewModel = remember(session) {
        val adapter = SquireApiAdapter(session.baseUrl, session.household, session.token)
        val outbox = RoomOutbox(db.outboxDao(), json)
        val store = PlayerStore(
            fetcher = adapter,
            cache = RoomStateCache(db.cacheDao()),
            outbox = outbox,
            sync = SyncEngine(outbox, adapter, adapter),
            json = json,
            ids = { ids.getAndIncrement() },
        )
        SquireViewModel(store)
    }

    val state by viewModel.state.collectAsStateWithLifecycle()
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    val context = LocalContext.current

    // Foreground auto-refresh (SQUIRE-T-0041): sync on a cadence while visible; never throws.
    LaunchedEffect(session) {
        lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                viewModel.refresh().join()
                delay(AUTO_REFRESH_MS)
            }
        }
    }

    // Self-heal a stale server address (SQUIRE-T-0052): while we can't reach the stored host (the
    // view is from cache, or errored), best-effort re-discover the server over mDNS; if it now lives
    // at a different host/port, update the session (keeping the token) so the adapter reconnects to
    // the new address — no re-pairing. No-op while online; the actual relocate needs a real LAN.
    LaunchedEffect(session) {
        while (true) {
            val s = viewModel.state.value
            val offline = (s is PlayerUiState.Ready && s.fromCache) || s is PlayerUiState.Error
            if (offline) {
                val found = discovery.discover()
                if (found != null && (found.first != session.host || found.second != session.port)) {
                    onSessionChanged(session.copy(host = found.first, port = found.second))
                    break // session changes → this effect re-keys against the new address
                }
            }
            delay(RELOCATE_INTERVAL_MS)
        }
    }

    // Server-distributed update check (SQUIRE-T-0051): once per session, surface a banner if the
    // server advertises a newer build. "Get update" opens the APK download in the browser.
    var update by remember(session) { mutableStateOf<UpdateInfo?>(null) }
    LaunchedEffect(session) {
        update = UpdateChecker.check(session.baseUrl, "squire", BuildConfig.VERSION_CODE)
    }

    Column(modifier = Modifier.fillMaxSize()) {
        update?.let { info ->
            UpdateBanner(info) {
                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(info.downloadUrl)))
            }
        }
        Box(modifier = Modifier.weight(1f)) {
            PlayerHomeScreen(
                state = state,
                onRefresh = { viewModel.refresh() },
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
                onForget = onForget,
            )
        }
    }
}
