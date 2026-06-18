package com.squire.app

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
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
import com.squire.app.ui.theme.SquireTheme
import com.squire.core.PlayerStore
import com.squire.core.PlayerUiState
import com.squire.core.SyncEngine
import com.squire.knight.app.KnightHomeHost
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

/** Wire role: the paired session's role that gets the Knight (parent) UI; everyone else is a Squire. */
private const val ROLE_KNIGHT = "Knight"

/**
 * The single household app (SQUIRE-T-0054). It is **session-gated** then **role-routed**: on first
 * run it pairs (ADR SQUIRE-A-0010); once a [Session] is stored, its **role** decides the UI — a
 * Knight session gets the parent review home, anyone else the child player home. There is no
 * separate "Knight app"; the trust boundary lives on the server (`RequireKnight` gates every
 * privileged call), so a Squire-paired device simply never renders — and can't successfully call —
 * the parent surface. "Forget" returns to pairing; a debug-only demo bypass keeps the emulator flow
 * one-tap (release always pairs).
 */
class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val sessionStore = SessionStore(this)
        val discovery = NsdDiscovery(this)

        setContent {
            SquireTheme {
                var session by remember { mutableStateOf(sessionStore.load()) }
                val current = session
                val onSessionChanged: (Session) -> Unit = { s -> sessionStore.save(s); session = s }
                val onForget: () -> Unit = { sessionStore.clear(); session = null }

                when {
                    current == null -> PairingScreen(
                        discovery = discovery,
                        demoLogin = if (BuildConfig.DEBUG) ({ demoLogin() }) else null,
                        onPaired = onSessionChanged,
                    )
                    current.role == ROLE_KNIGHT -> KnightHomeHost(
                        session = current,
                        discovery = discovery,
                        onSessionChanged = onSessionChanged,
                        onForget = onForget,
                    )
                    else -> PlayerHomeHost(
                        session = current,
                        discovery = discovery,
                        onSessionChanged = onSessionChanged,
                        onForget = onForget,
                    )
                }
            }
        }
    }

    /**
     * Debug-only bypass: log in with the demo creds against `squire-home` → a ready [Session]. The
     * demo pairs as the **Squire** (user 2); pair manually as user 1 to exercise the Knight UI.
     */
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

/**
 * The Squire (child) player home — the role-routed branch shown for a non-Knight session. Self-
 * contained: builds its own durable `squire.db` store from the session, runs auto-refresh (T-0041),
 * self-heals a stale address via mDNS (T-0052), and surfaces the update banner (T-0051).
 */
@Composable
internal fun PlayerHomeHost(
    session: Session,
    discovery: NsdDiscovery,
    onSessionChanged: (Session) -> Unit,
    onForget: () -> Unit,
) {
    val context = LocalContext.current
    val json = remember { Json { ignoreUnknownKeys = true } }
    val db = remember { SquireDb.build(context) }
    val ids = remember { AtomicLong(System.currentTimeMillis()) }

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

    // Foreground auto-refresh (SQUIRE-T-0041): sync on a cadence while visible; never throws.
    LaunchedEffect(session) {
        lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                viewModel.refresh().join()
                delay(AUTO_REFRESH_MS)
            }
        }
    }

    // Self-heal a stale server address (SQUIRE-T-0052): while unreachable, re-discover over mDNS and
    // update the session host/port (keeping the token) so the adapter reconnects. No-op while online.
    LaunchedEffect(session) {
        while (true) {
            val s = viewModel.state.value
            val offline = (s is PlayerUiState.Ready && s.fromCache) || s is PlayerUiState.Error
            if (offline) {
                val found = discovery.discover()
                if (found != null && (found.first != session.host || found.second != session.port)) {
                    onSessionChanged(session.copy(host = found.first, port = found.second))
                    break
                }
            }
            delay(RELOCATE_INTERVAL_MS)
        }
    }

    // Server-distributed update check (SQUIRE-T-0051).
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
