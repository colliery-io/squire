package com.squire.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import kotlinx.coroutines.launch
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.squire.app.ui.theme.SquireIcons
import androidx.compose.material3.Icon
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.LaunchedEffect
import com.squire.app.ui.components.Dot
import androidx.compose.ui.unit.sp
import com.squire.app.ui.theme.SquireGoldOn
import com.squire.app.ui.theme.SquireGoldFace
import com.squire.app.ui.theme.SquireDisplay
import com.squire.app.ui.components.WaxSeal
import com.squire.app.ui.components.SealSlot
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.size
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import com.squire.app.ui.theme.Tinctures
import com.squire.app.ui.theme.tinctureOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.squire.app.BuildConfig
import com.squire.app.ui.components.Banner
import com.squire.app.ui.components.CashPill
import com.squire.app.ui.components.GoldPill
import com.squire.app.ui.components.Medallion
import com.squire.app.ui.components.ProgressDots
import com.squire.app.ui.components.SectionTitle
import com.squire.app.ui.components.StatusChip
import com.squire.app.ui.theme.SquireGold
import com.squire.app.ui.theme.SquireTheme
import com.squire.core.PlayerUiState
import com.squire.sdk.model.BadgeView
import com.squire.sdk.model.AdjustmentView
import com.squire.sdk.model.ClaimStateKind
import com.squire.sdk.model.ActivityKind
import com.squire.sdk.model.GoalView
import com.squire.sdk.model.ClaimStatus
import com.squire.sdk.model.LockReasonKind
import com.squire.sdk.model.QuestCard
import com.squire.sdk.model.QuestStatus
import com.squire.sdk.model.RedemptionStateKind
import com.squire.sdk.model.RedemptionStatus
import com.squire.sdk.model.RewardCard
import com.squire.sdk.model.StateView
import com.squire.sdk.model.StreakView

/** The four pages of the child home, shown one at a time via the bottom navigation bar. */
enum class SquireTab(val label: String, val icon: androidx.compose.ui.graphics.vector.ImageVector) {
    Quests("Today", SquireIcons.Today),
    Rewards("Rewards", SquireIcons.Rewards),
    Me("Me", SquireIcons.Me),
    Activity("Activity", SquireIcons.Journal),
}

/**
 * The child's quest home — "playful quest" themed (SQUIRE-T-0056). Stateless: renders the whole
 * [PlayerUiState] and reports intents via callbacks; all logic lives in `:core`'s `PlayerStore`.
 *
 * Split into four bottom-nav pages (Quests / Rewards / Me / Activity) so each is focused rather than
 * one long scroll; the header (name + balance + menu) is pinned across all of them.
 *
 * @param headerLabel overrides the title (the Knight's "Acting as <name>" assume view, T-0055).
 * @param onBack when non-null, the top bar shows Back instead of Forget (assume mode).
 * @param initialTab screenshot/test seam — which page renders first.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PlayerHomeScreen(
    state: PlayerUiState,
    onRefresh: () -> Unit,
    onMarkDone: (questId: Long) -> Unit,
    onRedeem: (itemId: Long) -> Unit,
    onCashOut: (amount: Long) -> Unit = {},
    onPickTincture: ((String) -> Unit)? = null,
    onForget: () -> Unit = {},
    onCheckUpdate: () -> Unit = {},
    headerLabel: String? = null,
    onBack: (() -> Unit)? = null,
    initialTab: SquireTab = SquireTab.Quests,
    tappedQuestIds: Set<Long> = emptySet(), // screenshot seam: quests to render as just turned in
) {
    var tab by remember { mutableStateOf(initialTab) }
    val balance = (state as? PlayerUiState.Ready)?.view?.balance
    // Real-money "$ owed" (SQUIRE-T-0099): shown next to coins when the Squire is owed any dollars.
    val cashOwed = (state as? PlayerUiState.Ready)?.view?.balances.orEmpty()
        .firstOrNull { it.currency == com.squire.sdk.model.Currency.Cash }?.balance ?: 0L
    val name = headerLabel ?: "Squire"
    // Header greeting uses just the first name (split on spaces) — short + friendly, and keeps the
    // balance pills from getting crowded by a long full name.
    val firstName = name.substringBefore(' ')
    // Immediate "it registered!" feedback on every turn-in (usage feedback #5): a repeatable quest /
    // reward stays Available after a tap, so without this the child sees nothing happen and re-taps
    // (spams). The snackbar fires on tap regardless of the card's state.
    val snackbarHostState = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    fun cheer(msg: String) {
        scope.launch {
            snackbarHostState.currentSnackbarData?.dismiss() // replace, don't queue, on rapid taps
            snackbarHostState.showSnackbar(msg, duration = SnackbarDuration.Short)
        }
    }
    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        snackbarHost = { SnackbarHost(snackbarHostState) },
        bottomBar = {
            if (state is PlayerUiState.Ready) {
                NavigationBar(containerColor = MaterialTheme.colorScheme.surface) {
                    SquireTab.entries.forEach { t ->
                        NavigationBarItem(
                            selected = tab == t,
                            onClick = { tab = t },
                            icon = { Icon(t.icon, contentDescription = null) },
                            label = { Text(t.label) },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = MaterialTheme.colorScheme.onSurface,
                                selectedTextColor = MaterialTheme.colorScheme.onSurface,
                                unselectedIconColor = MaterialTheme.colorScheme.onSurface,
                                unselectedTextColor = MaterialTheme.colorScheme.onSurfaceVariant,
                                indicatorColor = MaterialTheme.colorScheme.surfaceVariant,
                            ),
                        )
                    }
                }
            }
        },
        topBar = {
            Column {
                TopAppBar(
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.primary,
                        titleContentColor = MaterialTheme.colorScheme.onPrimary,
                        actionIconContentColor = MaterialTheme.colorScheme.onPrimary,
                    ),
                    title = {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            // The initial sits on a white disc in the squire's own colour: a small
                            // crest against the tinted bar (SQUIRE-T-0136).
                            Surface(color = MaterialTheme.colorScheme.surface, shape = CircleShape) {
                                Text(
                                    firstName.take(1).uppercase(),
                                    color = MaterialTheme.colorScheme.primary,
                                    fontWeight = FontWeight.Bold,
                                    modifier = Modifier.padding(horizontal = 11.dp, vertical = 5.dp),
                                )
                            }
                            Spacer(Modifier.width(10.dp))
                            // Flex + ellipsize the name so a long "Hi, <name>!" yields space to the
                            // balance pills on the right rather than pushing them off-screen (clipped $).
                            Text(
                                when {
                                    tab != SquireTab.Quests -> tab.label
                                    name == "Squire" -> "Today"
                                    else -> "Hi, $firstName!"
                                },
                                fontWeight = FontWeight.Bold,
                                maxLines = 1,
                                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                                modifier = Modifier.weight(1f),
                            )
                            if (balance != null) {
                                Spacer(Modifier.width(8.dp))
                                GoldPill(balance, large = false)
                            }
                            if (cashOwed > 0) {
                                Spacer(Modifier.width(6.dp))
                                CashPill(cashOwed, large = false)
                            }
                        }
                    },
                    actions = {
                        var menuOpen by remember { mutableStateOf(false) }
                        IconButton(onClick = { menuOpen = true }) {
                            Text(
                                "⋮",
                                color = MaterialTheme.colorScheme.onPrimary,
                                style = MaterialTheme.typography.titleLarge,
                            )
                        }
                        DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                            DropdownMenuItem(
                                text = { Text("Refresh") },
                                onClick = { menuOpen = false; onRefresh() },
                            )
                            if (onBack != null) {
                                DropdownMenuItem(
                                    text = { Text("Back") },
                                    onClick = { menuOpen = false; onBack() },
                                )
                            } else {
                                DropdownMenuItem(
                                    text = { Text("Download latest update") },
                                    onClick = { menuOpen = false; onCheckUpdate() },
                                )
                                DropdownMenuItem(
                                    text = { Text("Forget this device") },
                                    onClick = { menuOpen = false; onForget() },
                                )
                            }
                            HorizontalDivider()
                            DropdownMenuItem(
                                text = {
                                    Text(
                                        "Squire v${BuildConfig.VERSION_NAME}",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                },
                                onClick = {},
                                enabled = false,
                            )
                        }
                    },
                )
            }
        },
    ) { padding ->
        when (state) {
            is PlayerUiState.Loading -> Centered(Modifier.fillMaxSize().padding(padding)) {
                CircularProgressIndicator()
                Spacer(Modifier.height(8.dp))
                Text("Loading…")
            }

            is PlayerUiState.Error -> Centered(Modifier.fillMaxSize().padding(padding)) {
                Text(state.message, color = MaterialTheme.colorScheme.error)
                Spacer(Modifier.height(12.dp))
                Button(onClick = onRefresh) { Text("Retry") }
            }

            is PlayerUiState.Ready -> ReadyContent(
                view = state.view,
                fromCache = state.fromCache,
                tab = tab,
                // No snackbar on a turn-in (SQUIRE-T-0133): the row itself moves to "waiting", coins
                // go ghost and the seal slot appears — the feedback IS the state change.
                onMarkDone = onMarkDone,
                onRedeem = { iid -> cheer("Sent! 🎁 Asked a grown-up"); onRedeem(iid) },
                onCashOut = { amt -> cheer("Sent! 💵 Asked a grown-up to pay you"); onCashOut(amt) },
                onPickTincture = onPickTincture,
                tappedQuestIds = tappedQuestIds,
                modifier = Modifier.fillMaxSize().padding(padding),
            )
        }
    }
}

@Composable
private fun ReadyContent(
    view: StateView,
    fromCache: Boolean,
    tab: SquireTab,
    onMarkDone: (questId: Long) -> Unit,
    onRedeem: (itemId: Long) -> Unit,
    onCashOut: (amount: Long) -> Unit,
    onPickTincture: ((String) -> Unit)?,
    modifier: Modifier = Modifier,
    tappedQuestIds: Set<Long> = emptySet(),
) {
    // Owed real money + whether a cash-out is already awaiting a grown-up (SQUIRE-T-0118).
    val cashOwed = view.balances.orEmpty()
        .firstOrNull { it.currency == com.squire.sdk.model.Currency.Cash }?.balance ?: 0L
    val cashoutPending = view.myCashouts.orEmpty()
        .any { it.state.state == com.squire.sdk.model.RedemptionStateKind.Pending }
    // Tap any quest / reward / streak / badge card to see its full details (SQUIRE-T-0094 #8).
    var detail by remember { mutableStateOf<DetailContent?>(null) }
    // Turn-ins the child has tapped since this view arrived (SQUIRE-T-0133). The store is not
    // optimistic — enqueue → sync → refresh — so until the next view lands (which may be a while,
    // offline) the tapped row shows "waiting" itself. A repeatable quest comes back Available with
    // the next view, which is right: it can be done again.
    val sent = remember { mutableStateMapOf<Long, Long>().also { m -> tappedQuestIds.forEach { id -> m[id] = view.generatedAt } } }
    fun tappedSince(view: StateView, questId: Long) = sent[questId] == view.generatedAt
    Box(modifier) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(if (tab == SquireTab.Activity) 2.dp else 10.dp),
        ) {
            // The offline banner rides on the Quests page (the landing page).
            if (fromCache && tab == SquireTab.Quests) {
                item {
                    // Offline is a state, not an error: quiet, not red (SQUIRE-T-0133).
                    Banner(
                        text = "Offline — showing your last saved quests",
                        container = MaterialTheme.colorScheme.surfaceVariant,
                        content = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            // Keys are namespaced ("q"/"i"/"s"…) because ids share one LazyColumn and a QuestId can
            // numerically equal an ItemId — an un-namespaced key would collide (SQUIRE-T-0070).
            when (tab) {
                SquireTab.Quests -> {
                    // What today is FOR (SQUIRE-T-0133): the nearest reward not yet affordable, with
                    // how far along the purse is. The strongest motivator in the app, so it leads.
                    savingFor(view)?.let { goal ->
                        item(key = "saving") { SavingForCard(goal, view.balance, onClick = { detail = rewardDetail(goal) }) }
                    }
                    // A live streak is worth a line; the pips are the same ones the Me tab draws.
                    view.streaks.filter { it.alive && it.current > 0 }.maxByOrNull { it.current }?.let { streak ->
                        item(key = "streak") { StreakLine(streak) }
                    }
                    item(key = "today-title") {
                        val sealed = view.questsToday.count { it.status == QuestStatus.CompletedToday }
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            SectionTitle("Today")
                            Spacer(Modifier.weight(1f))
                            if (view.questsToday.isNotEmpty()) {
                                Text(
                                    "$sealed of ${view.questsToday.size} sealed",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                    if (view.questsToday.isEmpty()) {
                        item { EmptyHint("No quests today — well done, brave Squire! 🎉") }
                    } else {
                        items(view.questsToday, key = { "q" + it.questId }) { q ->
                            QuestCardRow(
                                q,
                                notYet = notYetReason(view, q),
                                sending = tappedSince(view, q.questId),
                                onMarkDone = { id -> sent[id] = view.generatedAt; onMarkDone(id) },
                                onClick = { detail = questDetail(q) },
                            )
                        }
                    }
                }
                SquireTab.Rewards -> {
                    // "Cash out" the owed real money (SQUIRE-T-0118) — a redemption of the $ balance,
                    // shown above the coin shop whenever a grown-up owes the Squire dollars.
                    if (cashOwed > 0) {
                        item { CashOutRow(cashOwed, pending = cashoutPending, onCashOut = { onCashOut(cashOwed) }) }
                    }
                    item { SectionTitle("Rewards") }
                    if (view.rewards.isEmpty()) {
                        item { EmptyHint("No rewards in the shop yet — ask a grown-up! 🛒") }
                    } else {
                        items(view.rewards, key = { "i" + it.itemId }) { RewardCardRow(it, view.balance, onRedeem, onClick = { detail = rewardDetail(it) }) }
                    }
                }
                SquireTab.Me -> {
                    // My colours (SQUIRE-T-0136): pick the tincture the whole app wears. Hidden in
                    // assume mode (a Knight looking in), where there is no one to pick for.
                    if (onPickTincture != null) {
                        item { SectionTitle("My colours") }
                        item { TincturePicker(chosen = view.tincture, onPick = onPickTincture) }
                    }
                    val badges = view.badges.orEmpty()
                    val goals = view.goals.orEmpty()
                    if (badges.isEmpty() && view.streaks.isEmpty() && goals.isEmpty()) {
                        item { EmptyHint("Finish quests to earn badges, build streaks and unlock goals! 🌟") }
                    }
                    if (badges.isNotEmpty()) {
                        item { SectionTitle("🏅 Badges") }
                        items(badges, key = { "b" + it.id }) { BadgeCardRow(it, onClick = { detail = badgeDetail(it) }) }
                    }
                    if (view.streaks.isNotEmpty()) {
                        item { SectionTitle("Streaks") }
                        items(view.streaks, key = { "s" + it.name }) { StreakCardRow(it, onClick = { detail = streakDetail(it) }) }
                    }
                    if (goals.isNotEmpty()) {
                        item { SectionTitle("🎯 Goals to unlock") }
                        items(goals, key = { "g" + it.id }) { GoalCardRow(it, onClick = { detail = goalDetail(it) }) }
                    }
                }
                SquireTab.Activity -> {
                    item { SectionTitle("Recent activity") }
                    // One feed, already merged + newest-first by the server (SQUIRE-T-0124).
                    val activity = view.recentActivity.orEmpty()
                    if (activity.isNotEmpty()) {
                        itemsIndexed(activity, key = { i, _ -> "act$i" }) { _, entry ->
                            when (entry.kind) {
                                ActivityKind.Adjustment -> entry.adjustment?.let { AdjustmentRow(it) }
                                ActivityKind.Claim -> entry.claim?.let { ClaimRow(it) }
                                ActivityKind.Request -> entry.request?.let { RequestRow(it) }
                            }
                        }
                    } else {
                        // Fallback for an older server that doesn't send the merged feed: the legacy
                        // type-grouped lists.
                        val adjustments = view.adjustments.orEmpty()
                        if (view.myClaims.isEmpty() && view.myRequests.isEmpty() && adjustments.isEmpty()) {
                            item { EmptyHint("Nothing yet — go finish a quest! 💪") }
                        } else {
                            items(adjustments, key = { "adj${it.at}_${it.amount}" }) { AdjustmentRow(it) }
                            items(view.myClaims, key = { "c" + it.claimId }) { ClaimRow(it) }
                            items(view.myRequests, key = { "r" + it.requestId }) { RequestRow(it) }
                        }
                    }
                }
            }
        }
        detail?.let { DetailDialog(it) { detail = null } }
    }
}

/** The reward this Squire is closest to affording: the cheapest one they can't yet have. `null`
 *  when everything is affordable (or there are no rewards) — then there is nothing to save for. */
private fun savingFor(view: StateView): RewardCard? =
    view.rewards.filter { !it.affordable && it.lock == null && it.cost > view.balance }.minByOrNull { it.cost }

/** A grown-up said "not yet" to today's turn-in of this quest: the reason they gave, if any. Matched
 *  by title + day because a claim carries no quest id. */
private fun notYetReason(view: StateView, quest: QuestCard): String? {
    if (quest.status != QuestStatus.Available) return null
    val rejected = view.myClaims.lastOrNull {
        it.questTitle == quest.title && it.on == quest.on && it.state.state == ClaimStateKind.Rejected
    } ?: return null
    return rejected.state.reason?.takeIf { it.isNotBlank() } ?: "Have another go and try again."
}

@Composable
private fun SavingForCard(reward: RewardCard, balance: Int, onClick: () -> Unit) {
    val short = reward.cost - balance
    QuestCard(onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Saving for", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.weight(1f))
            Text(
                "$balance / ${reward.cost}",
                fontFamily = SquireDisplay, fontWeight = FontWeight.Bold, fontSize = 17.sp,
                color = SquireGoldOn,
            )
        }
        Spacer(Modifier.height(2.dp))
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(reward.icon ?: "🎁", fontSize = 20.sp)
            Text(reward.name, style = MaterialTheme.typography.titleLarge, maxLines = 1,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis)
        }
        Spacer(Modifier.height(10.dp))
        // Gold, because it is coins. The only bar in the app that is.
        LinearProgressIndicator(
            progress = { (balance.toFloat() / reward.cost).coerceIn(0f, 1f) },
            color = SquireGoldFace,
            trackColor = MaterialTheme.colorScheme.outlineVariant,
            strokeCap = StrokeCap.Round,
            gapSize = 0.dp,
            drawStopIndicator = {},
            modifier = Modifier.fillMaxWidth().height(12.dp).clip(CircleShape),
        )
        Spacer(Modifier.height(8.dp))
        Text(
            if (short == 1) "1 more coin." else "$short more coins.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun StreakLine(streak: StreakView) {
    val target = streak.nextMilestone ?: streak.current
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.padding(horizontal = 2.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            repeat(target.coerceAtMost(10)) { i -> Dot(filled = i < streak.current) }
        }
        Text(
            "${streak.current} ${if (streak.current == 1) "day" else "days"} running · ${streak.name}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/** A quest today, in one of three states the child can read at a glance (SQUIRE-T-0133):
 *  to do (an "I did it" button), waiting (ghost coins + an empty seal slot), sealed (the wax seal). */
@Composable
private fun QuestCardRow(quest: QuestCard, notYet: String?, sending: Boolean, onMarkDone: (Long) -> Unit, onClick: () -> Unit) {
    val sealed = quest.status == QuestStatus.CompletedToday
    val waiting = quest.status == QuestStatus.Pending || (sending && quest.status == QuestStatus.Available)
    // Did this row just go from waiting to sealed under our eyes? Then the seal stamps down.
    val lastStatus = remember { mutableStateOf(quest.status) }
    val justSealed = sealed && lastStatus.value == QuestStatus.Pending
    LaunchedEffect(quest.status) { lastStatus.value = quest.status }
    QuestCard(onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
            Medallion(quest.icon, quest.title)
            Spacer(Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    quest.title,
                    style = MaterialTheme.typography.titleMedium,
                    color = if (sealed || waiting) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.onSurface,
                    textDecoration = if (sealed) TextDecoration.LineThrough else null,
                )
                Spacer(Modifier.height(4.dp))
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    GoldPill(quest.reward, ghost = waiting)
                    Text(
                        when {
                            sealed -> "earned"
                            waiting -> "waiting for a grown-up's seal"
                            else -> ""
                        },
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                if (notYet != null && !waiting) {
                    Spacer(Modifier.height(6.dp))
                    Text(
                        "Not yet — $notYet",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }
            Spacer(Modifier.width(8.dp))
            when {
                waiting && !sealed -> SealSlot()
                quest.status == QuestStatus.Available -> Button(
                    onClick = { onMarkDone(quest.questId) },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.tertiary,
                        contentColor = MaterialTheme.colorScheme.onTertiary,
                    ),
                ) { Text(if (notYet != null) "Try again" else "I did it", fontWeight = FontWeight.Bold) }
                sealed -> WaxSeal(stamp = justSealed)
                else -> StatusChip(
                    "Taken",
                    MaterialTheme.colorScheme.surfaceVariant,
                    MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun RewardCardRow(reward: RewardCard, balance: Int, onRedeem: (Long) -> Unit, onClick: () -> Unit) {
    QuestCard(accent = MaterialTheme.colorScheme.secondary, onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
            Medallion(reward.icon, reward.name, reward = true)
            Spacer(Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(reward.name, style = MaterialTheme.typography.titleMedium)
                Spacer(Modifier.height(4.dp))
                GoldPill(reward.cost)
            }
            val lock = reward.lock
            when {
                lock != null -> when (lock.kind) {
                    LockReasonKind.OutOfStock -> StatusChip(
                        "Out of stock",
                        MaterialTheme.colorScheme.surfaceVariant,
                        MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    LockReasonKind.NeedsAchievement -> StatusChip(
                        "🔒 ${lock.name ?: "Locked"}",
                        MaterialTheme.colorScheme.surfaceVariant,
                        MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                reward.affordable -> Button(onClick = { onRedeem(reward.itemId) }) { Text("Redeem") }
                else -> StatusChip(
                    // Show how close the child is, not a flat "Need more coins".
                    "${(reward.cost - balance).coerceAtLeast(1)} more coins",
                    MaterialTheme.colorScheme.surfaceVariant,
                    MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/** The "Cash out" affordance on the Rewards page (SQUIRE-T-0118): redeem owed real money, parent-approved. */
@Composable
private fun CashOutRow(owed: Long, pending: Boolean, onCashOut: () -> Unit) {
    QuestCard(accent = MaterialTheme.colorScheme.primary) {
        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
            Text("💵", style = MaterialTheme.typography.headlineMedium, modifier = Modifier.width(48.dp))
            Spacer(Modifier.width(8.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text("Cash out", style = MaterialTheme.typography.titleMedium)
                Spacer(Modifier.height(4.dp))
                Text(
                    "A grown-up owes you \$$owed",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            if (pending) {
                StatusChip(
                    "⏳ asked",
                    MaterialTheme.colorScheme.surfaceVariant,
                    MaterialTheme.colorScheme.onSurfaceVariant,
                )
            } else {
                Button(onClick = onCashOut) { Text("Cash out \$$owed") }
            }
        }
    }
}

@Composable
private fun BadgeCardRow(badge: BadgeView, onClick: () -> Unit) {
    QuestCard(onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
            Medallion("🏅")
            Spacer(Modifier.width(12.dp))
            Text(badge.name, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            GoldPill(badge.bonus)
        }
    }
}

@Composable
private fun GoalCardRow(goal: GoalView, onClick: () -> Unit) {
    QuestCard(onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
            Medallion("🎯")
            Spacer(Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(goal.name, style = MaterialTheme.typography.titleMedium)
                Text(
                    goal.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                // Progress toward the goal, like a streak's dots — "2 / 5" (SQUIRE-T-0122).
                val target = goal.target ?: 0
                if (target > 0) {
                    Spacer(Modifier.height(6.dp))
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        ProgressDots(current = goal.current ?: 0, target = target)
                        Spacer(Modifier.width(10.dp))
                        Text(
                            "${goal.current ?: 0} / $target",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            if (goal.bonus > 0) {
                Spacer(Modifier.width(8.dp))
                GoldPill(goal.bonus)
            }
        }
    }
}

@Composable
private fun StreakCardRow(streak: StreakView, onClick: () -> Unit) {
    QuestCard(onClick = onClick) {
        Column(modifier = Modifier.fillMaxWidth()) {
            Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                Text("🏅 ${streak.name}", style = MaterialTheme.typography.titleMedium)
                Text("best ${streak.best}", style = MaterialTheme.typography.bodySmall)
            }
            Spacer(Modifier.height(8.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                ProgressDots(current = streak.current, target = streak.nextMilestone ?: streak.current.coerceAtLeast(1))
                Spacer(Modifier.width(10.dp))
                val goal = streak.nextMilestone?.let { " / $it" } ?: ""
                Text(
                    "${streak.current}$goal" + if (!streak.alive) "  · broken" else "",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun ClaimRow(claim: ClaimStatus) {
    when (claim.state.state) {
        ClaimStateKind.Approved -> CompactRow("✓", claim.questTitle, "+${claim.state.points ?: 0} 🪙", MaterialTheme.colorScheme.primary)
        ClaimStateKind.Rejected -> CompactRow("✗", claim.questTitle, "rejected", MaterialTheme.colorScheme.error)
        ClaimStateKind.Pending -> CompactRow("⏳", claim.questTitle, "pending", MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

/** A coin grant (+) or hazard penalty (−) a grown-up applied (SQUIRE-T-0096). */
@Composable
private fun AdjustmentRow(adj: AdjustmentView) {
    val positive = adj.amount >= 0
    val mag = kotlin.math.abs(adj.amount)
    CompactRow(
        icon = "🪙",
        what = adj.reason.ifBlank { if (positive) "Coins from a grown-up" else "Coins taken away" },
        amount = (if (positive) "+" else "−") + "$mag 🪙",
        amountColor = if (positive) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
    )
}

@Composable
private fun RequestRow(request: RedemptionStatus) {
    when (request.state.state) {
        RedemptionStateKind.Approved -> CompactRow("🎁", request.itemName, "−${request.cost} 🪙", MaterialTheme.colorScheme.onSurface)
        RedemptionStateKind.Rejected -> CompactRow("✗", request.itemName, "rejected", MaterialTheme.colorScheme.error)
        RedemptionStateKind.Pending -> CompactRow("⏳", request.itemName, "pending", MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

/** One compact activity line (usage feedback): a status icon + the "what" (quest/reward name) and
 * the "how much" (the coin delta or short state) — a dense row, not a full card. */
@Composable
private fun CompactRow(icon: String, what: String, amount: String, amountColor: androidx.compose.ui.graphics.Color) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier.fillMaxWidth().padding(vertical = 3.dp, horizontal = 2.dp),
    ) {
        Text(icon, modifier = Modifier.width(30.dp))
        Text(
            what,
            style = MaterialTheme.typography.bodyLarge,
            maxLines = 1,
            overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        Text(amount, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold, color = amountColor)
    }
}

/** Content for the tap-to-open detail dialog (SQUIRE-T-0094 #8). */
internal data class DetailContent(
    val icon: String?,
    val title: String,
    val description: String? = null,
    val lines: List<Pair<String, String>>,
    val note: String? = null,
)

/** The detail content rendered as a standalone card — the body of [DetailDialog] without the popup,
 *  so it can be previewed/snapshotted (Paparazzi can't capture Dialog windows). */
@Composable
internal fun DetailCard(content: DetailContent, modifier: Modifier = Modifier) {
    Surface(
        modifier = modifier,
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(20.dp),
        shadowElevation = 2.dp,
    ) {
        Column(modifier = Modifier.padding(20.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                content.icon?.let {
                    Text(it, style = MaterialTheme.typography.headlineSmall)
                    Spacer(Modifier.width(10.dp))
                }
                Text(content.title, fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleLarge)
            }
            content.description?.takeIf { it.isNotBlank() }?.let {
                Spacer(Modifier.height(8.dp))
                Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurface)
            }
            Spacer(Modifier.height(12.dp))
            content.lines.forEach { (label, value) ->
                Row(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 3.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(value, fontWeight = FontWeight.Medium)
                }
            }
            content.note?.let {
                Spacer(Modifier.height(10.dp))
                Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}

@Composable
private fun DetailDialog(content: DetailContent, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        confirmButton = { TextButton(onClick = onDismiss) { Text("Close") } },
        title = {
            Row(verticalAlignment = Alignment.CenterVertically) {
                content.icon?.let {
                    Text(it, style = MaterialTheme.typography.headlineSmall)
                    Spacer(Modifier.width(10.dp))
                }
                Text(content.title, fontWeight = FontWeight.Bold)
            }
        },
        text = {
            Column {
                content.description?.takeIf { it.isNotBlank() }?.let {
                    Text(it, style = MaterialTheme.typography.bodyMedium)
                    Spacer(Modifier.height(10.dp))
                }
                content.lines.forEach { (label, value) ->
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(vertical = 3.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text(value, fontWeight = FontWeight.Medium)
                    }
                }
                content.note?.let {
                    Spacer(Modifier.height(8.dp))
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        },
    )
}

internal fun questDetail(q: QuestCard) = DetailContent(
    icon = q.icon,
    title = q.title,
    description = q.description,
    lines = buildList {
        add("Reward" to "${q.reward} coins")
        q.category?.let { add("Category" to it) }
        add(
            "Status" to when (q.status) {
                QuestStatus.Available -> "Ready to do"
                QuestStatus.Pending -> "Waiting for a grown-up"
                QuestStatus.CompletedToday -> "Done today 🎉"
                QuestStatus.TakenByOther -> "Taken by someone else"
            },
        )
    },
    note = "Tap “Done” on the quest when you’ve finished it.",
)

internal fun rewardDetail(r: RewardCard) = DetailContent(
    icon = r.icon,
    title = r.name,
    description = r.description,
    lines = buildList {
        add("Cost" to "${r.cost} coins")
        val lock = r.lock
        when (lock?.kind) {
            LockReasonKind.NeedsAchievement -> add("Unlocks with" to "🏅 ${lock.name ?: "an achievement"}")
            LockReasonKind.OutOfStock -> add("Stock" to "Out of stock")
            null -> {}
        }
        add(
            "Can you get it now?" to when {
                lock != null -> "Not yet — it's locked"
                r.affordable -> "Yes! 🎉"
                else -> "Not yet — keep saving"
            },
        )
        r.lastRedeemed?.let { add("Last got it" to formatDate(it)) }
    },
    note = when {
        r.lock?.kind == LockReasonKind.NeedsAchievement ->
            "Earn the “${r.lock?.name}” badge to unlock this reward."
        r.lock?.kind == LockReasonKind.OutOfStock -> "It's out of stock for now — check back later."
        !r.affordable -> "Keep earning coins to afford this one!"
        else -> "Ask a grown-up to redeem it for you."
    },
)

internal fun streakDetail(s: StreakView) = DetailContent(
    icon = "🔥",
    title = s.name,
    lines = buildList {
        add("Current streak" to "${s.current} day${if (s.current == 1) "" else "s"}")
        add("Best ever" to "${s.best} day${if (s.best == 1) "" else "s"}")
        s.nextMilestone?.let {
            add("Next reward at" to "$it days")
            add("Days to go" to "${(it - s.current).coerceAtLeast(0)}")
        }
        add("Status" to if (s.alive) "Going strong 🔥" else "Broken — start again")
    },
    note = when {
        !s.alive -> "Do this today to start a fresh streak!"
        s.nextMilestone != null ->
            "Keep it going ${(s.nextMilestone!! - s.current).coerceAtLeast(1)} more day(s) to earn the next reward."
        else -> "Keep the streak alive — do it every day!"
    },
)

/** Short "MMM d" date for a millisecond epoch (e.g. a reward's last-redeemed). */
private fun formatDate(epochMs: Long): String =
    java.text.SimpleDateFormat("MMM d", java.util.Locale.getDefault()).format(java.util.Date(epochMs))

internal fun badgeDetail(b: BadgeView) = DetailContent(
    icon = "🏅",
    title = b.name,
    lines = buildList {
        if (b.bonus > 0) add("Bonus" to "+${b.bonus} coins")
        add("Earned" to "✓ Yes")
    },
)

internal fun goalDetail(g: GoalView) = DetailContent(
    icon = "🎯",
    title = g.name,
    // "How to earn it" reads as a sentence, so render it as a stacked, full-width block (the
    // description slot) instead of a cramped label↔value row, matching quest/reward detail
    // (SQUIRE-T-0122).
    description = g.description,
    lines = buildList {
        val target = g.target ?: 0
        if (target > 0) add("Progress" to "${g.current ?: 0} / $target")
        if (g.bonus > 0) add("Reward" to "+${g.bonus} coins")
    },
    note = "Not unlocked yet — keep going!",
)

/** Six swatches; the chosen one is ringed in ink. Tapping one recolours the app (SQUIRE-T-0136). */
@Composable
private fun TincturePicker(chosen: String?, onPick: (String) -> Unit) {
    val current = tinctureOf(chosen).name
    QuestCard {
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Tinctures.forEach { t ->
                val picked = t.name == current
                Box(
                    modifier = Modifier
                        .size(40.dp)
                        .then(if (picked) Modifier.border(3.dp, MaterialTheme.colorScheme.onSurface, CircleShape) else Modifier)
                        .padding(if (picked) 5.dp else 3.dp)
                        .clip(CircleShape)
                        .background(t.tint)
                        .clickable(enabled = !picked) { onPick(t.name) }
                        .semantics { contentDescription = t.name.replaceFirstChar { c -> c.uppercase() } },
                )
            }
        }
        Text(
            "Pick the colour your Squire wears.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 10.dp),
        )
    }
}

/** A plain card wrapper for quests/rewards/streaks. */
@Composable
private fun QuestCard(
    @Suppress("UNUSED_PARAMETER") accent: Color = MaterialTheme.colorScheme.outlineVariant,
    onClick: (() -> Unit)? = null,
    content: @Composable () -> Unit,
) {
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
        modifier = Modifier.fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable { onClick() } else Modifier),
    ) {
        // The coloured left stripe is gone (SQUIRE-T-0132): it restated what the row's own button or
        // chip already says. `accent` stays in the signature until SQUIRE-T-0133 rebuilds these rows.
        Column(modifier = Modifier.padding(14.dp)) { content() }
    }
}

@Composable
private fun EmptyHint(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(vertical = 4.dp),
    )
}

@Composable
private fun Centered(modifier: Modifier, content: @Composable () -> Unit) {
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) { content() }
}

@Preview(showBackground = true)
@Composable
private fun PlayerHomePreview() {
    val sample = StateView(
        balance = 12,
        generatedAt = 0L,
        squire = 1L,
        questsToday = listOf(
            QuestCard(on = 1, questId = 1L, reward = 5, status = QuestStatus.Available, title = "Make your bed", icon = "🛏"),
            QuestCard(on = 1, questId = 2L, reward = 10, status = QuestStatus.Pending, title = "Tidy your room", icon = "🧹"),
        ),
        rewards = listOf(
            RewardCard(affordable = true, cost = 3, itemId = 1L, name = "Ice cream", icon = "🍦"),
            RewardCard(affordable = false, cost = 15, itemId = 2L, name = "Movie night", icon = "🎬"),
        ),
        myClaims = emptyList(),
        myRequests = emptyList(),
        streaks = listOf(StreakView(name = "Room Master", current = 3, best = 5, alive = true, nextMilestone = 7)),
        badges = listOf(BadgeView(at = 0L, bonus = 25, id = 1L, name = "Century Club")),
    )
    SquireTheme {
        PlayerHomeScreen(
            state = PlayerUiState.Ready(sample, fromCache = false),
            onRefresh = {}, onMarkDone = {}, onRedeem = {},
        )
    }
}
