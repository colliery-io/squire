package com.squire.knight.app

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import com.squire.app.BuildConfig
import com.squire.app.ui.PlayerHomeScreen
import com.squire.core.PlayerUiState
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.knight.app.data.RoomPrivilegedOutbox
import com.squire.knight.app.data.RoomReviewCache
import com.squire.knight.app.data.db.KnightDb
import com.squire.knight.app.ui.KnightHomeScreen
import com.squire.knight.app.ui.KnightTab
import com.squire.knight.core.KnightStore
import com.squire.knight.core.KnightSyncEngine
import com.squire.knight.core.KnightUiState
import com.squire.pairing.NsdDiscovery
import com.squire.pairing.Session
import com.squire.pairing.UpdateBanner
import com.squire.pairing.UpdateChecker
import com.squire.pairing.UpdateInfo
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
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
 *
 * A Knight can also **assume any Squire** (SQUIRE-T-0055): "Open" on a Squire drops into that
 * Squire's player home; acting there goes through the Knight's privileged commands (audited as the
 * Knight, no impersonation token).
 */
@Composable
internal fun KnightHomeHost(
    session: Session,
    discovery: NsdDiscovery,
    onSessionChanged: (Session) -> Unit,
    onForget: () -> Unit,
    initialTab: KnightTab = KnightTab.Review,
) {
    val context = LocalContext.current
    val json = remember { Json { ignoreUnknownKeys = true } }
    val db = remember { KnightDb.build(context) }
    val ids = remember { AtomicLong(System.currentTimeMillis()) }

    // Hoisted so both the review store and the "assume Squire" read use the same Knight-token transport.
    val adapter = remember(session) {
        KnightApiAdapter(session.baseUrl, session.household, session.token)
    }
    val viewModel = remember(session) {
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
    // Activity feed for the History tab, fetched lazily when that tab is opened (SQUIRE-T-0118 polish).
    val scope = rememberCoroutineScope()
    var history by remember(session) { mutableStateOf<List<com.squire.sdk.model.HistoryEntryDto>>(emptyList()) }

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
    val updateContext = LocalContext.current
    LaunchedEffect(session) {
        update = UpdateChecker.check(updateContext, session.baseUrl, "squire", BuildConfig.VERSION_CODE)
    }

    // The Squire currently being "assumed" (id + display name), or null for the review home.
    var assumed by remember(session) { mutableStateOf<Pair<Long, String>?>(null) }
    // Whether the native quest-authoring surface (SQUIRE-T-0065) is open.
    var managingQuests by remember(session) { mutableStateOf(false) }
    // Whether the native achievement-authoring surface (SQUIRE-T-0072) is open.
    var managingAchievements by remember(session) { mutableStateOf(false) }
    // Whether the native reward-authoring surface (SQUIRE-T-0074) is open.
    var managingRewards by remember(session) { mutableStateOf(false) }
    // Whether the native member-administration surface (SQUIRE-T-0075) is open.
    var managingMembers by remember(session) { mutableStateOf(false) }
    // Whether the hazard catalog (SQUIRE-T-0096) is open.
    var managingHazards by remember(session) { mutableStateOf(false) }
    // Whether the household settings (timezone, SQUIRE-T-0113) surface is open.
    var managingSettings by remember(session) { mutableStateOf(false) }
    // Whether the household activity history (SQUIRE-T-0112) surface is open.
    var viewingHistory by remember(session) { mutableStateOf(false) }

    val current = assumed
    if (current != null) {
        AssumedSquireHome(
            adapter = adapter,
            viewModel = viewModel,
            lifecycle = lifecycle,
            squireId = current.first,
            squireName = current.second,
            onBack = { assumed = null },
        )
        return
    }

    if (managingQuests) {
        // The assignment picker reuses the squires already in the review.
        val squires = (state as? KnightUiState.Ready)
            ?.review
            ?.squires
            ?.map { it.squire to it.displayName }
            ?: emptyList()
        com.squire.knight.app.ui.QuestAdminScreen(
            adapter = adapter,
            squires = squires,
            onBack = { managingQuests = false },
        )
        return
    }

    if (managingAchievements) {
        com.squire.knight.app.ui.AchievementAdminScreen(
            adapter = adapter,
            onBack = { managingAchievements = false },
        )
        return
    }

    if (managingRewards) {
        com.squire.knight.app.ui.RewardAdminScreen(
            adapter = adapter,
            onBack = { managingRewards = false },
        )
        return
    }

    if (managingMembers) {
        com.squire.knight.app.ui.MemberAdminScreen(
            adapter = adapter,
            selfUser = session.user,
            host = session.host,
            port = session.port,
            household = session.household,
            onBack = { managingMembers = false },
        )
        return
    }

    if (managingHazards) {
        val squires = (state as? KnightUiState.Ready)
            ?.review?.squires?.map { it.squire to it.displayName } ?: emptyList()
        com.squire.knight.app.ui.HazardAdminScreen(
            adapter = adapter,
            squires = squires,
            onApply = { squire, amount, reason -> viewModel.adjust(squire, amount, reason, com.squire.sdk.model.Currency.Coins) },
            onBack = { managingHazards = false },
        )
        return
    }

    if (managingSettings) {
        com.squire.knight.app.ui.SettingsScreen(
            adapter = adapter,
            onBack = { managingSettings = false },
        )
        return
    }

    if (viewingHistory) {
        val names = (state as? KnightUiState.Ready)
            ?.review?.squires?.associate { it.squire to it.displayName } ?: emptyMap()
        com.squire.knight.app.ui.HistoryScreen(
            adapter = adapter,
            squireNames = names,
            onBack = { viewingHistory = false },
        )
        return
    }

    Column(modifier = Modifier.fillMaxSize()) {
        update?.let { info -> UpdateBanner(info) }
        Box(modifier = Modifier.weight(1f)) {
            KnightHomeScreen(
                state = state,
                initialTab = initialTab,
                name = session.displayName, // "Hi, <name>!" header (blank -> "Your Family")
                onRefresh = { viewModel.refresh() },
                onApproveClaim = { viewModel.approveClaim(it) },
                onRejectClaim = { id, reason -> viewModel.rejectClaim(id, reason) },
                onApproveRequest = { viewModel.approveRequest(it) },
                onRejectRequest = { id, reason -> viewModel.rejectRequest(id, reason) },
                onApproveCashOut = { viewModel.approveCashOut(it) },
                onRejectCashOut = { id, reason -> viewModel.rejectCashOut(id, reason) },
                history = history,
                onHistoryShown = {
                    scope.launch {
                        runCatching { adapter.history(limit = 100) }.onSuccess { history = it }
                    }
                },
                onAddFunds = { squire, amount, reason, currency -> viewModel.adjust(squire, amount, reason, currency) },
                onPay = { squire, amount, reason -> viewModel.pay(squire, amount, reason) },
                onRedeem = { squire, itemId -> viewModel.redeem(squire, itemId) },
                onMarkDone = { squire, questId, on -> viewModel.markDone(squire, questId, on) },
                onForget = onForget,
                onOpenSquire = { id, name -> assumed = id to name },
                onManageQuests = { managingQuests = true },
                onManageAchievements = { managingAchievements = true },
                onManageRewards = { managingRewards = true },
                onManageMembers = { managingMembers = true },
                onManageHazards = { managingHazards = true },
                onManageSettings = { managingSettings = true },
                onViewHistory = { viewingHistory = true },
            )
        }
    }
}

/**
 * "Acting as <Squire>" — a Knight operating a chosen Squire's player home (SQUIRE-T-0055). The view
 * is the child [PlayerHomeScreen] fed by the Knight-gated `GET /admin/squire/{id}/state` (T-0053);
 * its actions route through the Knight's privileged commands (mark-done / redeem **for** the Squire),
 * so they are audited as the Knight. Refetches after an action and on the auto-refresh cadence.
 */
@Composable
private fun AssumedSquireHome(
    adapter: KnightApiAdapter,
    viewModel: KnightViewModel,
    lifecycle: Lifecycle,
    squireId: Long,
    squireName: String,
    onBack: () -> Unit,
) {
    var view by remember(squireId) { mutableStateOf<PlayerUiState>(PlayerUiState.Loading) }
    var tick by remember(squireId) { mutableStateOf(0) }

    // Fetch the assumed Squire's state on entry, after each action, and on the refresh cadence.
    LaunchedEffect(squireId, tick) {
        view = try {
            PlayerUiState.Ready(adapter.squireState(squireId), fromCache = false)
        } catch (e: Exception) {
            PlayerUiState.Error("Couldn't load $squireName's home — is the computer reachable?")
        }
    }
    LaunchedEffect(squireId) {
        lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                delay(AUTO_REFRESH_MS)
                tick++
            }
        }
    }

    PlayerHomeScreen(
        state = view,
        onRefresh = { tick++ },
        onMarkDone = { questId ->
            val on = (view as? PlayerUiState.Ready)
                ?.view
                ?.questsToday
                ?.firstOrNull { it.questId == questId }
                ?.on
                ?: 0
            viewModel.markDone(squireId, questId, on)
            tick++
        },
        onRedeem = { itemId ->
            viewModel.redeem(squireId, itemId)
            tick++
        },
        headerLabel = "Acting as $squireName",
        onBack = onBack,
    )
}
