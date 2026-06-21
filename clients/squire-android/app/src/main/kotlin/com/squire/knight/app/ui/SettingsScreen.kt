package com.squire.knight.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.sdk.model.HouseholdConfig
import kotlinx.coroutines.launch

/**
 * Household settings for a Knight (SQUIRE-T-0113): read + change the timezone, at parity with the
 * Keep. The IANA zone sets the local-midnight day boundary for daily/weekly quests; a change takes
 * effect **live** (the server hot-swaps the shared config cell). An unknown zone is rejected by the
 * server (400) and surfaced inline. A handful of common zones are offered as quick-picks; any valid
 * IANA name can be typed.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
internal fun SettingsScreen(
    adapter: KnightApiAdapter,
    onBack: () -> Unit,
    initialConfig: HouseholdConfig? = null, // screenshot/test seam
) {
    androidx.activity.compose.BackHandler { onBack() }
    val scope = rememberCoroutineScope()

    var current by remember { mutableStateOf(initialConfig?.timezone ?: "") }
    var input by remember { mutableStateOf(initialConfig?.timezone ?: "") }
    var loadError by remember { mutableStateOf<String?>(null) }
    var saveError by remember { mutableStateOf<String?>(null) }
    var saved by remember { mutableStateOf(false) }

    androidx.compose.runtime.LaunchedEffect(Unit) {
        if (initialConfig != null) return@LaunchedEffect
        runCatching { adapter.getConfig() }
            .onSuccess { current = it.timezone; input = it.timezone; loadError = null }
            .onFailure { loadError = "Couldn't load settings — is the computer reachable?" }
    }

    val common = listOf(
        "UTC", "America/Detroit", "America/New_York", "America/Chicago",
        "America/Denver", "America/Los_Angeles", "Europe/London",
    )

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("Settings") },
                navigationIcon = { TextButton(onClick = onBack) { Text("Back") } },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                ),
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxWidth().padding(padding).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Time zone", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            Text(
                "Sets when each day starts for daily and weekly quests. Applies right away.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            loadError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            if (current.isNotBlank()) {
                Text("Current: $current", style = MaterialTheme.typography.bodyMedium)
            }
            OutlinedTextField(
                value = input,
                onValueChange = { input = it; saved = false; saveError = null },
                label = { Text("IANA zone (e.g. America/Detroit)") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                common.forEach { z ->
                    OutlinedButton(onClick = { input = z; saved = false; saveError = null }) { Text(z) }
                }
            }
            saveError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            if (saved) Text("Saved ✓", color = MaterialTheme.colorScheme.primary)
            Button(
                enabled = input.isNotBlank() && input != current,
                onClick = {
                    scope.launch {
                        runCatching { adapter.updateConfig(input.trim()) }
                            .onSuccess { current = it.timezone; input = it.timezone; saved = true; saveError = null }
                            .onFailure {
                                saveError = "Unknown time zone — check the spelling (e.g. America/Detroit)."
                            }
                    }
                },
            ) { Text("Save") }
        }
    }
}
