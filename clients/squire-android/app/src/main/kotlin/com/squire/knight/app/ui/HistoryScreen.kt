package com.squire.knight.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.sdk.model.Currency
import com.squire.sdk.model.HistoryEntryDto
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

/**
 * Read-only household activity history for a Knight (SQUIRE-T-0112): the recent feed of approvals,
 * rejections, redemptions, adjustments, payouts and unlocks across all Squires — at parity with the
 * Keep's event-log inspector, but a household-wide feed. Each row is rendered from a flattened
 * [HistoryEntryDto]; the Squire's display name is resolved from [squireNames] (the home review).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun HistoryScreen(
    adapter: KnightApiAdapter,
    squireNames: Map<Long, String>,
    onBack: () -> Unit,
    initialEntries: List<HistoryEntryDto>? = null, // screenshot/test seam
) {
    androidx.activity.compose.BackHandler { onBack() }

    var entries by remember { mutableStateOf(initialEntries ?: emptyList()) }
    var loadError by remember { mutableStateOf<String?>(null) }
    var loaded by remember { mutableStateOf(initialEntries != null) }

    LaunchedEffect(Unit) {
        if (initialEntries != null) return@LaunchedEffect
        runCatching { adapter.history(limit = 100) }
            .onSuccess { entries = it; loadError = null; loaded = true }
            .onFailure { loadError = "Couldn't load history — is the computer reachable?"; loaded = true }
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("History") },
                navigationIcon = { TextButton(onClick = onBack) { Text("Back") } },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                ),
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxWidth().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            loadError?.let { msg ->
                item { Text(msg, color = MaterialTheme.colorScheme.error) }
            }
            if (loaded && loadError == null && entries.isEmpty()) {
                item { Text("No activity yet.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            itemsIndexed(entries) { _, e ->
                HistoryRow(e, squireNames[e.squire] ?: "Squire ${e.squire}")
            }
        }
    }
}

/** One activity-feed row; reused by the Knight Review's History tab (SQUIRE-T-0118 polish). */
@Composable
internal fun HistoryRow(e: HistoryEntryDto, squireName: String) {
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(modifier = Modifier.fillMaxWidth().padding(12.dp)) {
            Text(describe(e, squireName), style = MaterialTheme.typography.bodyLarge)
            e.reason?.takeIf { it.isNotBlank() }?.let {
                Text("“$it”", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Text(
                formatWhen(e.at),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

private val whenFmt: DateTimeFormatter =
    DateTimeFormatter.ofLocalizedDateTime(FormatStyle.MEDIUM, FormatStyle.SHORT)

private fun formatWhen(atMillis: Long): String =
    runCatching {
        Instant.ofEpochMilli(atMillis).atZone(ZoneId.systemDefault()).format(whenFmt)
    }.getOrDefault("")

/** A human, present-tense one-liner for an activity entry. Coin/cash deltas are shown signed. */
private fun describe(e: HistoryEntryDto, name: String): String {
    val amt = e.amount
    return when (e.kind) {
        "Claimed" -> "$name claimed a quest"
        "Approved" -> "$name earned ${amt ?: 0} coins"
        "Rejected" -> "$name's quest claim was rejected"
        "Redeemed" -> "$name redeemed a reward (${amt ?: 0} coins)"
        "Requested" -> "$name requested a reward"
        "RedemptionRejected" -> "$name's reward request was rejected"
        "CashOutRequested" -> "$name asked to cash out $${amt ?: 0}"
        "CashedOut" -> "$name cashed out $${kotlin.math.abs(amt ?: 0)}"
        "CashOutRejected" -> "$name's cash-out was declined"
        "Unlocked" -> "$name unlocked an achievement (+${amt ?: 0})"
        "Adjusted" -> {
            val unit = if (e.currency == Currency.Cash) "$" else "coins"
            val n = amt ?: 0
            val verb = if (n >= 0) "received" else "lost"
            val magnitude = if (e.currency == Currency.Cash) "$unit${kotlin.math.abs(n)}" else "${kotlin.math.abs(n)} $unit"
            "$name $verb $magnitude"
        }
        else -> "$name · ${e.kind}"
    }
}
