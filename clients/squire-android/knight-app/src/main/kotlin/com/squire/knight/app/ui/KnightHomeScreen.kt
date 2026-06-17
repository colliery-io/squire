package com.squire.knight.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.squire.knight.core.KnightUiState
import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.ItemOption
import com.squire.sdk.model.PendingClaim
import com.squire.sdk.model.PendingRequest
import com.squire.sdk.model.QuestOption
import com.squire.sdk.model.SquireSummary

/**
 * Stateless Knight review home: renders the whole [KnightUiState] (the cross-Squire
 * [HouseholdReview]) and reports the parent's quick-actions back through callbacks. All logic lives
 * in `:knight-core`'s `KnightStore`.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KnightHomeScreen(
    state: KnightUiState,
    onRefresh: () -> Unit,
    onApproveClaim: (claimId: Long) -> Unit,
    onRejectClaim: (claimId: Long) -> Unit,
    onApproveRequest: (requestId: Long) -> Unit,
    onRejectRequest: (requestId: Long) -> Unit,
    onAddFunds: (squire: Long, amount: Long, reason: String) -> Unit,
    onRedeem: (squire: Long, itemId: Long) -> Unit,
    onMarkDone: (squire: Long, questId: Long, on: Int) -> Unit,
    onForget: () -> Unit = {},
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Knight — review", fontWeight = FontWeight.Bold) },
                actions = {
                    TextButton(onClick = onRefresh) { Text("Refresh") }
                    TextButton(onClick = onForget) { Text("Forget") }
                },
            )
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

            is KnightUiState.Ready -> ReadyContent(
                review = state.review,
                fromCache = state.fromCache,
                onApproveClaim = onApproveClaim,
                onRejectClaim = onRejectClaim,
                onApproveRequest = onApproveRequest,
                onRejectRequest = onRejectRequest,
                onAddFunds = onAddFunds,
                onRedeem = onRedeem,
                onMarkDone = onMarkDone,
                modifier = Modifier.fillMaxSize().padding(padding),
            )
        }
    }
}

@Composable
private fun ReadyContent(
    review: HouseholdReview,
    fromCache: Boolean,
    onApproveClaim: (Long) -> Unit,
    onRejectClaim: (Long) -> Unit,
    onApproveRequest: (Long) -> Unit,
    onRejectRequest: (Long) -> Unit,
    onAddFunds: (Long, Long, String) -> Unit,
    onRedeem: (Long, Long) -> Unit,
    onMarkDone: (Long, Long, Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    // The Squire a pending claim/request belongs to is labelled by display name from the summaries.
    val names = review.squires.associate { it.squire to it.displayName }

    var fundsFor by remember { mutableStateOf<SquireSummary?>(null) }
    var redeemFor by remember { mutableStateOf<SquireSummary?>(null) }
    var markDoneFor by remember { mutableStateOf<SquireSummary?>(null) }

    LazyColumn(
        modifier = modifier,
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (fromCache) item { OfflineBanner() }

        item { SectionHeader("Squires") }
        if (review.squires.isEmpty()) {
            item { Text("No Squires yet.") }
        } else {
            items(review.squires, key = { it.squire }) { s ->
                SquireRow(
                    s = s,
                    onAddFunds = { fundsFor = s },
                    onRedeem = { redeemFor = s },
                    onMarkDone = { markDoneFor = s },
                )
            }
        }

        item { SectionHeader("Pending claims") }
        if (review.pendingClaims.isEmpty()) {
            item { Text("Nothing to review.") }
        } else {
            items(review.pendingClaims, key = { it.claimId }) { claim ->
                PendingClaimRow(
                    claim = claim,
                    squireName = names[claim.squire] ?: "Squire ${claim.squire}",
                    onApprove = { onApproveClaim(claim.claimId) },
                    onReject = { onRejectClaim(claim.claimId) },
                )
            }
        }

        item { SectionHeader("Pending redemption requests") }
        if (review.pendingRequests.isEmpty()) {
            item { Text("Nothing to review.") }
        } else {
            items(review.pendingRequests, key = { it.requestId }) { req ->
                PendingRequestRow(
                    request = req,
                    squireName = names[req.squire] ?: "Squire ${req.squire}",
                    onApprove = { onApproveRequest(req.requestId) },
                    onReject = { onRejectRequest(req.requestId) },
                )
            }
        }
    }

    fundsFor?.let { target ->
        AddFundsDialog(
            squireName = target.displayName,
            onDismiss = { fundsFor = null },
            onConfirm = { amount, reason ->
                onAddFunds(target.squire, amount, reason)
                fundsFor = null
            },
        )
    }

    redeemFor?.let { target ->
        PickDialog(
            title = "Redeem for ${target.displayName}",
            empty = "No items to redeem.",
            options = review.items.map { it.itemId to "${it.name} · ${it.cost} pts" },
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
}

@Composable
private fun SquireRow(
    s: SquireSummary,
    onAddFunds: () -> Unit,
    onRedeem: () -> Unit,
    onMarkDone: () -> Unit,
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Text(s.displayName, fontWeight = FontWeight.Medium)
        Text("${s.balance} pts", style = MaterialTheme.typography.bodySmall)
        Row(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.padding(top = 4.dp),
        ) {
            OutlinedButton(onClick = onMarkDone) { Text("Mark done") }
            OutlinedButton(onClick = onRedeem) { Text("Redeem") }
            OutlinedButton(onClick = onAddFunds) { Text("Add funds") }
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
    Column(modifier = Modifier.fillMaxWidth()) {
        Text(claim.questTitle, fontWeight = FontWeight.Medium)
        Text("$squireName · day ${claim.on}", style = MaterialTheme.typography.bodySmall)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(top = 4.dp)) {
            Button(onClick = onApprove) { Text("Approve") }
            OutlinedButton(onClick = onReject) { Text("Reject") }
        }
    }
}

@Composable
private fun PendingRequestRow(request: PendingRequest, squireName: String, onApprove: () -> Unit, onReject: () -> Unit) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Text(request.itemName, fontWeight = FontWeight.Medium)
        Text("$squireName · ${request.cost} pts", style = MaterialTheme.typography.bodySmall)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(top = 4.dp)) {
            Button(onClick = onApprove) { Text("Approve") }
            OutlinedButton(onClick = onReject) { Text("Reject") }
        }
    }
}

@Composable
private fun AddFundsDialog(squireName: String, onDismiss: () -> Unit, onConfirm: (Long, String) -> Unit) {
    var amount by remember { mutableStateOf("") }
    var reason by remember { mutableStateOf("") }
    val parsedAmount = amount.toLongOrNull()
    // REQ-K6: a positive amount AND a non-empty reason are required before the action is allowed.
    val valid = parsedAmount != null && parsedAmount > 0 && reason.isNotBlank()

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add funds — $squireName") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = amount,
                    onValueChange = { amount = it.filter(Char::isDigit) },
                    label = { Text("Amount (points)") },
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
                Text("Add")
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun OfflineBanner() {
    Surface(color = MaterialTheme.colorScheme.errorContainer, modifier = Modifier.fillMaxWidth()) {
        Text(
            "Offline — showing last saved review",
            color = MaterialTheme.colorScheme.onErrorContainer,
            modifier = Modifier.padding(12.dp),
        )
    }
}

@Composable
private fun SectionHeader(text: String) {
    Column {
        HorizontalDivider()
        Text(
            text,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.padding(top = 8.dp),
        )
    }
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
    KnightHomeScreen(
        state = KnightUiState.Ready(sample, fromCache = false),
        onRefresh = {}, onApproveClaim = {}, onRejectClaim = {},
        onApproveRequest = {}, onRejectRequest = {}, onAddFunds = { _, _, _ -> },
        onRedeem = { _, _ -> }, onMarkDone = { _, _, _ -> },
    )
}
