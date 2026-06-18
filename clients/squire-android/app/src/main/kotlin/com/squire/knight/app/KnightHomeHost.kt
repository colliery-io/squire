package com.squire.knight.app

import android.content.Intent
import android.net.Uri
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
import com.squire.app.BuildConfig
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.knight.app.data.RoomPrivilegedOutbox
import com.squire.knight.app.data.RoomReviewCache
import com.squire.knight.app.data.db.KnightDb
import com.squire.knight.app.ui.KnightHomeScreen
import com.squire.knight.core.KnightStore
import com.squire.knight.core.KnightSyncEngine
import com.squire.knight.core.KnightUiState
import com.squire.pairing.NsdDiscovery
import com.squire.pairing.Session
import com.squire.pairing.UpdateBanner
import com.squire.pairing.UpdateChecker
import com.squire.pairing.UpdateInfo
import kotlinx.coroutines.delay
import kotlinx.serialization.json.Json
import java.util.concurrent.atomic.AtomicLong

/** Foreground auto-refresh cadence (SQUIRE-T-0041). */
private const val AUTO_REFRESH_MS = 5_000L

/** How often to attempt mDNS re-discovery while the server is unreachable (SQUIRE-T-0052). */
private const val RELOCATE_INTERVAL_MS = 15_000L

/**
 * The Knight (parent) review home — the role-routed branch of the merged app (SQUIRE-T-0054), shown
 * when the paired [Session] role is `Knight`. Self-contained: builds its own durable `knight.db`
 * store from the session's host + Knight token, runs auto-refresh (T-0041), self-heals a stale
 * address via mDNS (T-0052), and surfaces the update banner (T-0051). "Forget" clears the session.
 */
@Composable
internal fun KnightHomeHost(
    session: Session,
    discovery: NsdDiscovery,
    onSessionChanged: (Session) -> Unit,
    onForget: () -> Unit,
) {
    val context = LocalContext.current
    val json = remember { Json { ignoreUnknownKeys = true } }
    val db = remember { KnightDb.build(context) }
    val ids = remember { AtomicLong(System.currentTimeMillis()) }

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

    // Self-heal a stale server address (SQUIRE-T-0052): while unreachable, re-discover over mDNS and
    // update the session host/port (keeping the token) so the adapter reconnects. No-op while online.
    LaunchedEffect(session) {
        while (true) {
            val s = viewModel.state.value
            val offline = (s is KnightUiState.Ready && s.fromCache) || s is KnightUiState.Error
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

    // Server-distributed update check (SQUIRE-T-0051): one app now, so the manifest key is "squire".
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
    }
}
