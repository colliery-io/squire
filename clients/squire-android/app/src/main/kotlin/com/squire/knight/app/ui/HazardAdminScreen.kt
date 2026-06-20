package com.squire.knight.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.sdk.model.Hazard
import kotlinx.coroutines.launch

/**
 * Hazard catalog authoring + quick-apply for a Knight (SQUIRE-T-0096): define named penalties (a
 * negative behavior + how many coins it costs), and tap one to apply it to a Squire — which sends a
 * negative `AdjustPoints` (reason = the hazard name) via the shared adjust path. The catalog is
 * household config (shared with the Keep). Floors at zero; the deduction shows in the child's
 * activity feed with the reason. Styled to the "playful quest" theme.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
internal fun HazardAdminScreen(
    adapter: KnightApiAdapter,
    squires: List<Pair<Long, String>>,
    onApply: (squire: Long, amount: Long, reason: String) -> Unit,
    onBack: () -> Unit,
    initialHazards: List<Hazard>? = null, // screenshot/test seam
) {
    androidx.activity.compose.BackHandler { onBack() }
    val scope = rememberCoroutineScope()

    var hazards by remember { mutableStateOf(initialHazards ?: emptyList()) }
    var loadError by remember { mutableStateOf<String?>(null) }

    androidx.compose.runtime.LaunchedEffect(Unit) {
        if (initialHazards != null) return@LaunchedEffect
        runCatching { adapter.listHazards() }
            .onSuccess { hazards = it; loadError = null }
            .onFailure { loadError = "Couldn't load hazards — is the computer reachable?" }
    }

    fun persist(next: List<Hazard>) {
        hazards = next
        scope.launch { runCatching { adapter.setHazards(next) } }
    }

    var name by remember { mutableStateOf("") }
    var penalty by remember { mutableStateOf("5") }
    var formError by remember { mutableStateOf<String?>(null) }

    // The hazard currently being applied (kid picker), if any.
    var applying by remember { mutableStateOf<Hazard?>(null) }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("⚠ Hazards", style = MaterialTheme.typography.titleLarge) },
                navigationIcon = { TextButton(onClick = onBack) { Text("Back") } },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primary,
                    titleContentColor = MaterialTheme.colorScheme.onPrimary,
                    navigationIconContentColor = MaterialTheme.colorScheme.onPrimary,
                ),
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxWidth().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            item {
                Text(
                    "Define negative behaviors that cost coins. Tap a hazard to apply it — it can't take a Squire below zero, and shows in their activity with the reason.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            loadError?.let { item { Text(it, color = MaterialTheme.colorScheme.error) } }

            // ── add a hazard ──
            item {
                Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
                    Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("New hazard", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
                        OutlinedTextField(
                            value = name,
                            onValueChange = { name = it },
                            label = { Text("Behavior (e.g. Didn't brush teeth)") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        OutlinedTextField(
                            value = penalty,
                            onValueChange = { penalty = it.filter(Char::isDigit) },
                            label = { Text("Coins lost") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        formError?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
                        Button(
                            onClick = {
                                val p = penalty.toIntOrNull()
                                if (name.isBlank()) { formError = "Name the behavior."; return@Button }
                                if (p == null || p < 1) { formError = "Coins lost must be at least 1."; return@Button }
                                formError = null
                                persist(hazards + Hazard(name = name.trim(), penalty = p, icon = null))
                                name = ""; penalty = "5"
                            },
                            modifier = Modifier.fillMaxWidth(),
                        ) { Text("Add hazard") }
                    }
                }
            }

            item { Text("Hazards", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary) }
            if (hazards.isEmpty()) {
                item { Text("No hazards yet.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
            } else {
                items(hazards, key = { it.name }) { hazard ->
                    Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
                        Row(
                            Modifier.fillMaxWidth().padding(14.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Column(Modifier.weight(1f)) {
                                Text(hazard.name, style = MaterialTheme.typography.titleMedium)
                                Text("−${hazard.penalty} coins", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                            }
                            Button(onClick = { applying = hazard }) { Text("Apply") }
                            TextButton(onClick = { persist(hazards.filterNot { it.name == hazard.name }) }) { Text("Remove") }
                        }
                    }
                }
            }
        }
    }

    // ── apply: pick which Squire ──
    val target = applying
    if (target != null) {
        AlertDialog(
            onDismissRequest = { applying = null },
            confirmButton = {},
            dismissButton = { TextButton(onClick = { applying = null }) { Text("Cancel") } },
            title = { Text("Apply “${target.name}”") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Take ${target.penalty} coins from which Squire?")
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        squires.forEach { (id, squireName) ->
                            OutlinedButton(onClick = {
                                onApply(id, -target.penalty.toLong(), target.name)
                                applying = null
                            }) { Text(squireName) }
                        }
                    }
                    if (squires.isEmpty()) Text("No Squires found.", color = MaterialTheme.colorScheme.error)
                }
            },
        )
    }
}
