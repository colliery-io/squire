package com.squire.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.squire.core.PlayerUiState
import com.squire.sdk.model.ClaimStateKind
import com.squire.sdk.model.ClaimStatus
import com.squire.sdk.model.LockReasonKind
import com.squire.sdk.model.QuestCard
import com.squire.sdk.model.QuestStatus
import com.squire.sdk.model.RedemptionStatus
import com.squire.sdk.model.RewardCard
import com.squire.sdk.model.StateView
import com.squire.sdk.model.StreakView

/**
 * Stateless player-home screen: renders the whole [PlayerUiState] and reports user
 * intents back through callbacks. All logic lives in `:core`'s `PlayerStore`.
 *
 * @param onMarkDone invoked with a quest's id when the child taps "Mark done".
 * @param onRedeem invoked with a reward's id when the child taps "Redeem".
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PlayerHomeScreen(
    state: PlayerUiState,
    onRefresh: () -> Unit,
    onMarkDone: (questId: Long) -> Unit,
    onRedeem: (itemId: Long) -> Unit,
) {
    val balance = (state as? PlayerUiState.Ready)?.view?.balance
    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        if (balance != null) "Squire — $balance pts" else "Squire",
                        fontWeight = FontWeight.Bold,
                    )
                },
                actions = {
                    TextButton(onClick = onRefresh) { Text("Refresh") }
                },
            )
        },
    ) { padding ->
        when (state) {
            is PlayerUiState.Loading -> Column(
                modifier = Modifier.fillMaxSize().padding(padding),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                CircularProgressIndicator()
                Spacer(Modifier.height(8.dp))
                Text("Loading…")
            }

            is PlayerUiState.Error -> Column(
                modifier = Modifier.fillMaxSize().padding(padding),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(state.message, color = MaterialTheme.colorScheme.error)
                Spacer(Modifier.height(12.dp))
                Button(onClick = onRefresh) { Text("Retry") }
            }

            is PlayerUiState.Ready -> ReadyContent(
                view = state.view,
                fromCache = state.fromCache,
                onMarkDone = onMarkDone,
                onRedeem = onRedeem,
                modifier = Modifier.fillMaxSize().padding(padding),
            )
        }
    }
}

@Composable
private fun ReadyContent(
    view: StateView,
    fromCache: Boolean,
    onMarkDone: (questId: Long) -> Unit,
    onRedeem: (itemId: Long) -> Unit,
    modifier: Modifier = Modifier,
) {
    LazyColumn(
        modifier = modifier,
        contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (fromCache) {
            item { OfflineBanner() }
        }

        item { SectionHeader("Today's quests") }
        if (view.questsToday.isEmpty()) {
            item { Text("No quests today.") }
        } else {
            items(view.questsToday, key = { it.questId }) { quest ->
                QuestRow(quest = quest, onMarkDone = onMarkDone)
            }
        }

        item { SectionHeader("Rewards") }
        if (view.rewards.isEmpty()) {
            item { Text("No rewards available.") }
        } else {
            items(view.rewards, key = { it.itemId }) { reward ->
                RewardRow(reward = reward, onRedeem = onRedeem)
            }
        }

        item { SectionHeader("Streaks") }
        if (view.streaks.isEmpty()) {
            item { Text("No streaks yet.") }
        } else {
            items(view.streaks, key = { it.name }) { streak ->
                StreakRow(streak)
            }
        }

        item { SectionHeader("Recent claims") }
        if (view.myClaims.isEmpty()) {
            item { Text("No recent claims.") }
        } else {
            items(view.myClaims, key = { it.claimId }) { claim ->
                ClaimRow(claim)
            }
        }

        item { SectionHeader("Recent requests") }
        if (view.myRequests.isEmpty()) {
            item { Text("No recent requests.") }
        } else {
            items(view.myRequests, key = { it.requestId }) { request ->
                RequestRow(request)
            }
        }
    }
}

@Composable
private fun OfflineBanner() {
    Surface(
        color = MaterialTheme.colorScheme.errorContainer,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(
            "Offline — showing last saved view",
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
private fun QuestRow(quest: QuestCard, onMarkDone: (Long) -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(quest.title, fontWeight = FontWeight.Medium)
            Text("+${quest.reward} pts", style = MaterialTheme.typography.bodySmall)
        }
        when (quest.status) {
            QuestStatus.Available -> Button(onClick = { onMarkDone(quest.questId) }) {
                Text("Mark done")
            }
            QuestStatus.Pending -> StatusLabel("Pending review")
            QuestStatus.CompletedToday -> StatusLabel("Done today")
            QuestStatus.TakenByOther -> StatusLabel("Taken")
        }
    }
}

@Composable
private fun RewardRow(reward: RewardCard, onRedeem: (Long) -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(reward.name, fontWeight = FontWeight.Medium)
            Text("${reward.cost} pts", style = MaterialTheme.typography.bodySmall)
        }
        val lock = reward.lock
        when {
            lock != null -> when (lock.kind) {
                LockReasonKind.OutOfStock -> StatusLabel("Out of stock")
                LockReasonKind.NeedsAchievement ->
                    StatusLabel("Locked: ${lock.name ?: "achievement"}")
            }
            reward.affordable -> Button(onClick = { onRedeem(reward.itemId) }) { Text("Redeem") }
            else -> StatusLabel("Can't afford")
        }
    }
}

@Composable
private fun StreakRow(streak: StreakView) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Text(streak.name, fontWeight = FontWeight.Medium)
        val milestone = streak.nextMilestone?.let { " · next $it" } ?: ""
        val alive = if (streak.alive) "" else " · broken"
        Text(
            "current ${streak.current} / best ${streak.best}$milestone$alive",
            style = MaterialTheme.typography.bodySmall,
        )
    }
}

@Composable
private fun ClaimRow(claim: ClaimStatus) {
    val state = claim.state
    val label = when (state.state) {
        ClaimStateKind.Pending -> "Pending"
        ClaimStateKind.Approved -> "Approved" + (state.points?.let { " (+$it)" } ?: "")
        ClaimStateKind.Rejected -> "Rejected" + (state.reason?.let { ": $it" } ?: "")
    }
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(claim.questTitle, modifier = Modifier.weight(1f))
        Text(label, style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
private fun RequestRow(request: RedemptionStatus) {
    val state = request.state
    val label = state.state.value + (state.reason?.let { ": $it" } ?: "")
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(request.itemName, modifier = Modifier.weight(1f))
        Text("${request.cost} pts · $label", style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
private fun StatusLabel(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Preview(showBackground = true)
@Composable
private fun PlayerHomePreview() {
    val sample = StateView(
        balance = 120,
        generatedAt = 0L,
        squire = 1L,
        questsToday = listOf(
            QuestCard(on = 1, questId = 1L, reward = 10, status = QuestStatus.Available, title = "Make bed"),
            QuestCard(on = 1, questId = 2L, reward = 15, status = QuestStatus.TakenByOther, title = "Dishes"),
        ),
        rewards = listOf(
            RewardCard(affordable = true, cost = 50, itemId = 1L, name = "Ice cream"),
            RewardCard(affordable = false, cost = 500, itemId = 2L, name = "New game"),
        ),
        myClaims = listOf(
            ClaimStatus(
                claimId = 9L,
                on = 1,
                questTitle = "Make bed",
                state = com.squire.sdk.model.ClaimState(state = ClaimStateKind.Pending),
            ),
        ),
        myRequests = emptyList(),
        streaks = listOf(
            StreakView(alive = true, best = 9, current = 3, name = "Chores", nextMilestone = 5),
        ),
    )
    MaterialTheme {
        PlayerHomeScreen(
            state = PlayerUiState.Ready(sample, fromCache = true),
            onRefresh = {},
            onMarkDone = {},
            onRedeem = {},
        )
    }
}
