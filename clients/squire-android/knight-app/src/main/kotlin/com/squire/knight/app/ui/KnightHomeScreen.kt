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
import com.squire.sdk.model.PendingClaim
import com.squire.sdk.model.PendingRequest
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
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Knight — review", fontWeight = FontWeight.Bold) },
                actions = { TextButton(onClick = onRefresh) { Text("Refresh") } },
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
    modifier: Modifier = Modifier,
) {
    // The Squire a pending claim/request belongs to is labelled by display name from the summaries.
    val names = review.squires.associate { it.squire to it.displayName }

    var fundsFor by remember { mutableStateOf<SquireSummary?>(null) }

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
                SquireRow(s, onAddFunds = { fundsFor = s })
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
}

@Composable
private fun SquireRow(s: SquireSummary, onAddFunds: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(s.displayName, fontWeight = FontWeight.Medium)
            Text("${s.balance} pts", style = MaterialTheme.typography.bodySmall)
        }
        OutlinedButton(onClick = onAddFunds) { Text("Add funds") }
    }
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
    )
    KnightHomeScreen(
        state = KnightUiState.Ready(sample, fromCache = false),
        onRefresh = {}, onApproveClaim = {}, onRejectClaim = {},
        onApproveRequest = {}, onRejectRequest = {}, onAddFunds = { _, _, _ -> },
    )
}
