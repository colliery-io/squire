package com.squire.knight.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Surface
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.squire.app.ui.components.CashPill
import com.squire.app.ui.components.GoldPill
import com.squire.app.ui.components.Medallion
import com.squire.app.ui.components.SectionTitle
import com.squire.app.ui.components.StatusChip
import com.squire.app.ui.theme.SquireGold
import com.squire.app.ui.theme.SquireTheme
import com.squire.knight.core.KnightUiState
import com.squire.sdk.model.Currency
import com.squire.sdk.model.HistoryEntryDto
import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.ItemOption
import com.squire.sdk.model.PendingClaim
import com.squire.sdk.model.PendingCashOut
import com.squire.sdk.model.PendingRequest
import com.squire.sdk.model.QuestOption
import com.squire.sdk.model.SquireSummary

/**
 * Stateless Knight review home: renders the whole [KnightUiState] (the cross-Squire
 * [HouseholdReview]) and reports the parent's quick-actions back through callbacks. All logic lives
 * in `:knight-core`'s `KnightStore`. Styled to the "playful quest" theme (SQUIRE-T-0057): parchment
 * cards, royal serif headers, gold balance pills — kept scannable for a parent triaging the queue.
 */
/** The Knight (parent) home's bottom-nav pages — mirrors the Squire's tabbed layout for consistency. */
enum class KnightTab(val label: String, val icon: String) {
    Review("Review", "📋"),
    Manage("Manage", "🛠"),
    History("History", "📜"),
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KnightHomeScreen(
    state: KnightUiState,
    name: String = "",
    initialTab: KnightTab = KnightTab.Review,
    onRefresh: () -> Unit,
    onApproveClaim: (claimId: Long) -> Unit,
    onRejectClaim: (claimId: Long, reason: String?) -> Unit,
    onApproveRequest: (requestId: Long) -> Unit,
    onRejectRequest: (requestId: Long, reason: String?) -> Unit,
    onApproveCashOut: (requestId: Long) -> Unit = {},
    onRejectCashOut: (requestId: Long, reason: String?) -> Unit = { _, _ -> },
    onAddFunds: (squire: Long, amount: Long, reason: String, currency: Currency) -> Unit,
    onPay: (squire: Long, amount: Long, reason: String) -> Unit,
    onRedeem: (squire: Long, itemId: Long) -> Unit,
    onMarkDone: (squire: Long, questId: Long, on: Int) -> Unit,
    onForget: () -> Unit = {},
    onOpenSquire: (squire: Long, name: String) -> Unit = { _, _ -> },
    onManageQuests: () -> Unit = {},
    onManageAchievements: () -> Unit = {},
    onManageRewards: () -> Unit = {},
    onManageMembers: () -> Unit = {},
    onManageHazards: () -> Unit = {},
    onManageSettings: () -> Unit = {},
    onViewHistory: () -> Unit = {},
    history: List<HistoryEntryDto> = emptyList(),
    onHistoryShown: () -> Unit = {},
) {
    val firstName = name.substringBefore(' ')
    var tab by remember { mutableStateOf(initialTab) }
    // Lazily load the activity feed the first time the History tab is opened (SQUIRE-T-0118 polish).
    LaunchedEffect(tab) { if (tab == KnightTab.History) onHistoryShown() }
    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
          Column {
            TopAppBar(
                title = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Surface(color = MaterialTheme.colorScheme.secondary, shape = CircleShape) {
                            Text("🛡", modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp))
                        }
                        Spacer(Modifier.width(10.dp))
                        Text(
                            if (firstName.isBlank()) "Your Family" else "Hi, $firstName!",
                            style = MaterialTheme.typography.titleLarge,
                            fontWeight = FontWeight.Bold,
                            maxLines = 1,
                            overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primary,
                    titleContentColor = MaterialTheme.colorScheme.onPrimary,
                    actionIconContentColor = MaterialTheme.colorScheme.onPrimary,
                ),
                actions = {
                    var menuOpen by remember { mutableStateOf(false) }
                    IconButton(onClick = { menuOpen = true }) {
                        Text("⋮", color = MaterialTheme.colorScheme.onPrimary, style = MaterialTheme.typography.titleLarge)
                    }
                    // Manage surfaces + History moved to the bottom-nav tabs; the menu keeps the rest.
                    DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                        DropdownMenuItem(text = { Text("Settings") }, onClick = { menuOpen = false; onManageSettings() })
                        DropdownMenuItem(text = { Text("Refresh") }, onClick = { menuOpen = false; onRefresh() })
                        DropdownMenuItem(text = { Text("Forget device") }, onClick = { menuOpen = false; onForget() })
                    }
                },
            )
          }
        },
        bottomBar = {
            if (state is KnightUiState.Ready) {
                NavigationBar(containerColor = MaterialTheme.colorScheme.surface) {
                    KnightTab.entries.forEach { t ->
                        NavigationBarItem(
                            selected = tab == t,
                            onClick = { tab = t },
                            icon = { Text(t.icon, style = MaterialTheme.typography.titleLarge) },
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
    ) { padding ->
        when (state) {
            is KnightUiState.Loading -> Centered(Modifier.fillMaxSize().padding(padding)) {
                CircularProgressIndicator()
                Spacer(Modifier.height(8.dp))
                Text("Loading…")
            }

            is KnightUiState.Error -> Centered(Modifier.fillMaxSize().padding(padding)) {
                Text(state.message, color = MaterialTheme.colorScheme.error)
                Spacer(Modifier.height(12.dp))
                Button(onClick = onRefresh) { Text("Retry") }
            }

            is KnightUiState.Ready -> when (tab) {
                KnightTab.Review -> ReadyContent(
                    review = state.review,
                    fromCache = state.fromCache,
                    onApproveClaim = onApproveClaim,
                    onRejectClaim = onRejectClaim,
                    onApproveRequest = onApproveRequest,
                    onRejectRequest = onRejectRequest,
                    onApproveCashOut = onApproveCashOut,
                    onRejectCashOut = onRejectCashOut,
                    onAddFunds = onAddFunds,
                    onPay = onPay,
                    onRedeem = onRedeem,
                    onMarkDone = onMarkDone,
                    onOpenSquire = onOpenSquire,
                    modifier = Modifier.fillMaxSize().padding(padding),
                )
                KnightTab.Manage -> ManageHub(
                    onManageQuests = onManageQuests,
                    onManageRewards = onManageRewards,
                    onManageAchievements = onManageAchievements,
                    onManageHazards = onManageHazards,
                    onManageMembers = onManageMembers,
                    modifier = Modifier.fillMaxSize().padding(padding),
                )
                KnightTab.History -> HistoryTabPage(
                    history = history,
                    squireNames = state.review.squires.associate { it.squire to it.displayName },
                    modifier = Modifier.fillMaxSize().padding(padding),
                )
            }
        }
    }
}

/** The "Manage" tab: a hub of the parent's authoring/admin surfaces (each opens its full screen). */
@Composable
private fun ManageHub(
    onManageQuests: () -> Unit,
    onManageRewards: () -> Unit,
    onManageAchievements: () -> Unit,
    onManageHazards: () -> Unit,
    onManageMembers: () -> Unit,
    modifier: Modifier = Modifier,
) {
    LazyColumn(
        modifier = modifier,
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        item { SectionTitle("Manage") }
        item { ManageRow("⚔", "Quests", "Create, edit, schedule chores", onManageQuests) }
        item { ManageRow("🎁", "Rewards", "The shop your Squires spend coins in", onManageRewards) }
        item { ManageRow("🏅", "Achievements", "Badges, streaks and goals to unlock", onManageAchievements) }
        item { ManageRow("⚠", "Hazards", "Named penalties to apply", onManageHazards) }
        item { ManageRow("👪", "Members & devices", "Add people, pair phones", onManageMembers) }
    }
}

@Composable
private fun ManageRow(icon: String, title: String, subtitle: String, onClick: () -> Unit) {
    Card(
        onClick = onClick,
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth().padding(16.dp)) {
            Text(icon, style = MaterialTheme.typography.headlineSmall, modifier = Modifier.width(40.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                Text(subtitle, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Text("›", style = MaterialTheme.typography.headlineSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

/** The "History" tab: the household activity feed inline (SQUIRE-T-0118 polish) — approvals, rewards,
 *  cash-outs, adjustments and unlocks across all Squires, newest first. */
@Composable
private fun HistoryTabPage(
    history: List<HistoryEntryDto>,
    squireNames: Map<Long, String>,
    modifier: Modifier = Modifier,
) {
    if (history.isEmpty()) {
        Centered(modifier) {
            Text("📜", style = MaterialTheme.typography.displaySmall)
            Spacer(Modifier.height(8.dp))
            Text("No activity yet", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            Spacer(Modifier.height(4.dp))
            Text(
                "Approvals, rewards and adjustments will show up here.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        return
    }
    LazyColumn(
        modifier = modifier,
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        items(history) { e ->
            HistoryRow(e, squireNames[e.squire] ?: "Squire ${e.squire}")
        }
    }
}

@Composable
private fun ReadyContent(
    review: HouseholdReview,
    fromCache: Boolean,
    onApproveClaim: (Long) -> Unit,
    onRejectClaim: (Long, String?) -> Unit,
    onApproveRequest: (Long) -> Unit,
    onRejectRequest: (Long, String?) -> Unit,
    onApproveCashOut: (Long) -> Unit,
    onRejectCashOut: (Long, String?) -> Unit,
    onAddFunds: (Long, Long, String, Currency) -> Unit,
    onPay: (Long, Long, String) -> Unit,
    onRedeem: (Long, Long) -> Unit,
    onMarkDone: (Long, Long, Int) -> Unit,
    onOpenSquire: (Long, String) -> Unit,
    modifier: Modifier = Modifier,
) {
    // The Squire a pending claim/request belongs to is labelled by display name from the summaries.
    val names = review.squires.associate { it.squire to it.displayName }

    var fundsFor by remember { mutableStateOf<SquireSummary?>(null) }
    var payFor by remember { mutableStateOf<SquireSummary?>(null) }
    var redeemFor by remember { mutableStateOf<SquireSummary?>(null) }
    var markDoneFor by remember { mutableStateOf<SquireSummary?>(null) }
    // Pending reject awaiting a reason (SQUIRE-T-0078): the claim / request the Knight tapped Reject on.
    var rejectClaimFor by remember { mutableStateOf<PendingClaim?>(null) }
    var rejectRequestFor by remember { mutableStateOf<PendingRequest?>(null) }
    var rejectCashOutFor by remember { mutableStateOf<PendingCashOut?>(null) }

    val cashouts = review.pendingCashouts.orEmpty()
    val pendingCount = review.pendingClaims.size + review.pendingRequests.size + cashouts.size

    LazyColumn(
        modifier = modifier,
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (fromCache) item { OfflineBanner() }

        item {
            SectionTitle(
                if (pendingCount == 0) "Your Squires" else "Your Squires · $pendingCount awaiting your seal",
            )
        }
        if (review.squires.isEmpty()) {
            item { EmptyNote("No Squires yet. Pair a child device to add one.") }
        } else {
            items(review.squires, key = { it.squire }) { s ->
                SquireRow(
                    s = s,
                    onOpen = { onOpenSquire(s.squire, s.displayName) },
                    onAddFunds = { fundsFor = s },
                    onPay = { payFor = s },
                    onRedeem = { redeemFor = s },
                    onMarkDone = { markDoneFor = s },
                )
            }
        }

        item { SectionTitle("Quests to approve") }
        if (review.pendingClaims.isEmpty()) {
            item { EmptyNote("All caught up — no quests waiting.") }
        } else {
            items(review.pendingClaims, key = { it.claimId }) { claim ->
                PendingClaimRow(
                    claim = claim,
                    squireName = names[claim.squire] ?: "Squire ${claim.squire}",
                    onApprove = { onApproveClaim(claim.claimId) },
                    onReject = { rejectClaimFor = claim },
                )
            }
        }

        item { SectionTitle("Rewards to grant") }
        if (review.pendingRequests.isEmpty()) {
            item { EmptyNote("No reward requests waiting.") }
        } else {
            items(review.pendingRequests, key = { it.requestId }) { req ->
                PendingRequestRow(
                    request = req,
                    squireName = names[req.squire] ?: "Squire ${req.squire}",
                    onApprove = { onApproveRequest(req.requestId) },
                    onReject = { rejectRequestFor = req },
                )
            }
        }

        item { SectionTitle("Cash-outs to pay") }
        if (cashouts.isEmpty()) {
            item { EmptyNote("No cash-outs waiting.") }
        } else {
            items(cashouts, key = { "co" + it.requestId }) { co ->
                PendingCashOutRow(
                    cashout = co,
                    squireName = names[co.squire] ?: "Squire ${co.squire}",
                    onApprove = { onApproveCashOut(co.requestId) },
                    onReject = { rejectCashOutFor = co },
                )
            }
        }
    }

    fundsFor?.let { target ->
        AddFundsDialog(
            squireName = target.displayName,
            onDismiss = { fundsFor = null },
            onConfirm = { amount, reason, currency ->
                onAddFunds(target.squire, amount, reason, currency)
                fundsFor = null
            },
        )
    }

    payFor?.let { target ->
        PayDialog(
            squireName = target.displayName,
            owed = target.cashBalance ?: 0,
            onDismiss = { payFor = null },
            onConfirm = { amount, reason ->
                onPay(target.squire, amount, reason)
                payFor = null
            },
        )
    }

    redeemFor?.let { target ->
        PickDialog(
            title = "Redeem for ${target.displayName}",
            empty = "No items to redeem.",
            options = review.items.map { it.itemId to "${it.name} · ${it.cost} coins" },
            onPick = { itemId ->
                onRedeem(target.squire, itemId)
                redeemFor = null
            },
            onDismiss = { redeemFor = null },
        )
    }

    markDoneFor?.let { target ->
        PickDialog(
            title = "Mark done for ${target.displayName}",
            empty = "No quests to mark.",
            options = review.quests.map { it.questId to it.title },
            onPick = { questId ->
                onMarkDone(target.squire, questId, review.today)
                markDoneFor = null
            },
            onDismiss = { markDoneFor = null },
        )
    }

    rejectClaimFor?.let { claim ->
        RejectReasonDialog(
            title = "Reject quest",
            subject = "${claim.questTitle} · ${names[claim.squire] ?: "Squire ${claim.squire}"}",
            onDismiss = { rejectClaimFor = null },
            onConfirm = { reason ->
                onRejectClaim(claim.claimId, reason)
                rejectClaimFor = null
            },
        )
    }

    rejectRequestFor?.let { req ->
        RejectReasonDialog(
            title = "Reject reward",
            subject = "${req.itemName} · ${names[req.squire] ?: "Squire ${req.squire}"}",
            onDismiss = { rejectRequestFor = null },
            onConfirm = { reason ->
                onRejectRequest(req.requestId, reason)
                rejectRequestFor = null
            },
        )
    }

    rejectCashOutFor?.let { co ->
        RejectReasonDialog(
            title = "Reject cash-out",
            subject = "\$${co.amount} · ${names[co.squire] ?: "Squire ${co.squire}"}",
            onDismiss = { rejectCashOutFor = null },
            onConfirm = { reason ->
                onRejectCashOut(co.requestId, reason)
                rejectCashOutFor = null
            },
        )
    }
}

/**
 * Collect an **optional** rejection reason (SQUIRE-T-0078). A blank reason rejects with no note (the
 * engine treats the reason as optional); a typed reason reaches the child's "Recent" as
 * "Rejected: <reason>". `internal` so the screenshot harness can render it.
 */
@Composable
internal fun RejectReasonDialog(
    title: String,
    subject: String,
    onDismiss: () -> Unit,
    onConfirm: (String?) -> Unit,
) {
    var reason by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(subject, style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.SemiBold)
                OutlinedTextField(
                    value = reason,
                    onValueChange = { reason = it },
                    label = { Text("Reason (the child will see this)") },
                    placeholder = { Text("Optional") },
                    singleLine = false,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            Button(
                onClick = { onConfirm(reason.trim().ifBlank { null }) },
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.error,
                    contentColor = MaterialTheme.colorScheme.onError,
                ),
            ) { Text("Reject") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SquireRow(
    s: SquireSummary,
    onOpen: () -> Unit,
    onAddFunds: () -> Unit,
    onPay: () -> Unit,
    onRedeem: () -> Unit,
    onMarkDone: () -> Unit,
) {
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            // Row 1: name + Open (the name flexes/ellipsizes so it never pushes Open off).
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    "⚔ ${s.displayName}",
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.primary,
                    maxLines = 1,
                    overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                // Drop into this Squire's home and act on their behalf (SQUIRE-T-0055).
                FilledTonalButton(onClick = onOpen) { Text("Open") }
            }
            // Row 2: currency (coins + any real-money owed).
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                GoldPill(amount = s.balance.toInt())
                val owed = s.cashBalance ?: 0
                if (owed > 0) CashPill(owed.toLong(), large = false)
            }
            // Row 3: the same actions for every squire (uniform cards). Cash payout is NOT here —
            // "cashing out" is modelled as a redemption (drawing down the owed-cash balance, parent-
            // approved), handled through the rewards/redemption flow, so the card never grows a
            // variable extra button.
            // Tighter than Material's 24dp default: Lexend runs wider than Roboto, and at the default
            // the third button wrapped — making every squire card a row taller and pushing the
            // approval queue further down the screen.
            val tight = PaddingValues(horizontal = 14.dp, vertical = 8.dp)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = onMarkDone, contentPadding = tight) { Text("Mark done") }
                OutlinedButton(onClick = onRedeem, contentPadding = tight) { Text("Redeem") }
                OutlinedButton(onClick = onAddFunds, contentPadding = tight) { Text("Add funds") }
            }
        }
    }
}

/**
 * A simple single-pick dialog: each option is a tappable row that fires [onPick] with its id and
 * dismisses. Used for the Knight's direct-redeem (items) and mark-done (quests) pickers.
 */
@Composable
private fun PickDialog(
    title: String,
    empty: String,
    options: List<Pair<Long, String>>,
    onPick: (Long) -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            if (options.isEmpty()) {
                Text(empty)
            } else {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    options.forEach { (id, label) ->
                        TextButton(onClick = { onPick(id) }, modifier = Modifier.fillMaxWidth()) {
                            Text(label, modifier = Modifier.fillMaxWidth())
                        }
                    }
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun PendingClaimRow(claim: PendingClaim, squireName: String, onApprove: () -> Unit, onReject: () -> Unit) {
    ReviewCard(
        title = claim.questTitle,
        subtitle = "$squireName · day ${claim.on}",
        leading = { Medallion(null, claim.questTitle) },
        trailing = { StatusChip("Quest", MaterialTheme.colorScheme.primaryContainer, MaterialTheme.colorScheme.onPrimaryContainer) },
        onApprove = onApprove,
        onReject = onReject,
    )
}

@Composable
private fun PendingRequestRow(request: PendingRequest, squireName: String, onApprove: () -> Unit, onReject: () -> Unit) {
    ReviewCard(
        title = request.itemName,
        subtitle = "$squireName · ${request.cost} coins",
        leading = { Medallion(null, request.itemName, reward = true) },
        trailing = { StatusChip("Reward", MaterialTheme.colorScheme.secondaryContainer, MaterialTheme.colorScheme.onSecondaryContainer) },
        onApprove = onApprove,
        onReject = onReject,
    )
}

/** A pending cash-out (SQUIRE-T-0118): approve pays out the owed dollars; reject leaves them owed. */
@Composable
private fun PendingCashOutRow(cashout: PendingCashOut, squireName: String, onApprove: () -> Unit, onReject: () -> Unit) {
    ReviewCard(
        title = "Cash out \$${cashout.amount}",
        subtitle = "$squireName · real money owed",
        leading = { Text("💵", style = MaterialTheme.typography.headlineMedium) },
        trailing = { StatusChip("Cash", MaterialTheme.colorScheme.tertiaryContainer, MaterialTheme.colorScheme.onTertiaryContainer) },
        onApprove = onApprove,
        onReject = onReject,
    )
}

/** A parchment card for one pending item: title + who/when, a kind chip, and Approve / Reject. */
@Composable
private fun ReviewCard(
    title: String,
    subtitle: String,
    leading: @Composable () -> Unit,
    trailing: @Composable () -> Unit,
    onApprove: () -> Unit,
    onReject: () -> Unit,
) {
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(modifier = Modifier.fillMaxWidth().padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                leading()
                Spacer(Modifier.width(12.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(title, fontWeight = FontWeight.SemiBold)
                    Text(subtitle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                trailing()
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(top = 12.dp)) {
                Button(
                    onClick = onApprove,
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.tertiary,
                        contentColor = MaterialTheme.colorScheme.onTertiary,
                    ),
                ) { Text("Approve ✓") }
                OutlinedButton(onClick = onReject) { Text("Reject") }
            }
        }
    }
}

@Composable
private fun AddFundsDialog(squireName: String, onDismiss: () -> Unit, onConfirm: (Long, String, Currency) -> Unit) {
    var amount by remember { mutableStateOf("") }
    var reason by remember { mutableStateOf("") }
    // Coins (in-app) or Dollars (real money owed) — both go through the same Adjust command (T-0099).
    var currency by remember { mutableStateOf(Currency.Coins) }
    val isCash = currency == Currency.Cash
    val parsedAmount = amount.toLongOrNull()
    // REQ-K6: a positive amount AND a non-empty reason are required before the action is allowed.
    val valid = parsedAmount != null && parsedAmount > 0 && reason.isNotBlank()

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add to $squireName") },
        text = {
            AddFundsFields(
                amount = amount,
                onAmount = { amount = it.filter(Char::isDigit) },
                reason = reason,
                onReason = { reason = it },
                currency = currency,
                onCurrency = { currency = it },
            )
        },
        confirmButton = {
            Button(enabled = valid, onClick = { onConfirm(parsedAmount!!, reason.trim(), currency) }) {
                Text("Add")
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

/** The body of [AddFundsDialog]: the 🪙 Coins / $ Dollars toggle + amount + reason. Stateless so it
 *  can be previewed/snapshotted (Paparazzi can't capture the AlertDialog popup). */
@Composable
internal fun AddFundsFields(
    amount: String,
    onAmount: (String) -> Unit,
    reason: String,
    onReason: (String) -> Unit,
    currency: Currency,
    onCurrency: (Currency) -> Unit,
) {
    val isCash = currency == Currency.Cash
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FilterChip(selected = !isCash, onClick = { onCurrency(Currency.Coins) }, label = { Text("🪙 Coins") })
            FilterChip(selected = isCash, onClick = { onCurrency(Currency.Cash) }, label = { Text("\$ Dollars") })
        }
        OutlinedTextField(
            value = amount,
            onValueChange = onAmount,
            label = { Text(if (isCash) "Amount (dollars)" else "Amount (coins)") },
            singleLine = true,
        )
        OutlinedTextField(
            value = reason,
            onValueChange = onReason,
            label = { Text("Reason (required)") },
            singleLine = true,
        )
    }
}

/**
 * Settle real-money cash owed to a Squire (SQUIRE-T-0111), mirroring "Add coins". A positive amount
 * (capped at [owed]) and a non-empty reason are required; the amount pre-fills to the full owed
 * balance (the common case: pay it all off).
 */
@Composable
private fun PayDialog(squireName: String, owed: Int, onDismiss: () -> Unit, onConfirm: (Long, String) -> Unit) {
    var amount by remember { mutableStateOf(owed.toString()) }
    var reason by remember { mutableStateOf("") }
    val parsedAmount = amount.toLongOrNull()
    val valid = parsedAmount != null && parsedAmount > 0 && parsedAmount <= owed.toLong() && reason.isNotBlank()

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Pay $squireName") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Owed: \$$owed", style = MaterialTheme.typography.bodyMedium)
                OutlinedTextField(
                    value = amount,
                    onValueChange = { amount = it.filter(Char::isDigit) },
                    label = { Text("Amount ($, up to $owed)") },
                    singleLine = true,
                )
                OutlinedTextField(
                    value = reason,
                    onValueChange = { reason = it },
                    label = { Text("Reason (required)") },
                    singleLine = true,
                )
            }
        },
        confirmButton = {
            Button(enabled = valid, onClick = { onConfirm(parsedAmount!!, reason.trim()) }) {
                Text("Pay")
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun OfflineBanner() {
    com.squire.app.ui.components.Banner(
        text = "Offline — showing last saved review",
        container = MaterialTheme.colorScheme.errorContainer,
        content = MaterialTheme.colorScheme.onErrorContainer,
    )
}

@Composable
private fun EmptyNote(text: String) {
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
private fun KnightHomePreview() {
    val sample = HouseholdReview(
        generatedAt = 0L,
        squires = listOf(SquireSummary(balance = 25, displayName = "Gawain", squire = 2)),
        pendingClaims = listOf(PendingClaim(claimId = 9L, on = 20624, questTitle = "Tidy your room", squire = 2)),
        pendingRequests = listOf(PendingRequest(cost = 15, itemName = "Movie night", requestId = 5L, squire = 2)),
        items = listOf(ItemOption(itemId = 200, name = "Ice cream", cost = 3)),
        quests = listOf(QuestOption(questId = 100, title = "Make your bed")),
        today = 20624,
    )
    SquireTheme {
        KnightHomeScreen(
            state = KnightUiState.Ready(sample, fromCache = false),
            onRefresh = {}, onApproveClaim = {}, onRejectClaim = { _, _ -> },
            onApproveRequest = {}, onRejectRequest = { _, _ -> }, onAddFunds = { _, _, _, _ -> },
            onPay = { _, _, _ -> },
            onRedeem = { _, _ -> }, onMarkDone = { _, _, _ -> },
        )
    }
}
