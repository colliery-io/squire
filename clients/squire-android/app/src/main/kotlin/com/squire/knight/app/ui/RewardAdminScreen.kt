package com.squire.knight.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.Spacer
import androidx.compose.material3.FilledTonalButton
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.squire.app.ui.components.GoldPill
import com.squire.app.ui.components.Medallion
import com.squire.app.ui.components.SectionTitle
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.sdk.model.AvailabilityKind
import com.squire.sdk.model.CreateItemReq
import com.squire.sdk.model.ItemSummaryDto
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/** One entry from the bundled reward starter library (`assets/rewards-library.json`).
 * `internal` so the screenshot harness (T-0070) can build sample rows. */
@Serializable
internal data class LibraryReward(
    val name: String,
    val cost: Long,
    val availability: String = "Repeatable", // "Once" | "Repeatable"
    val description: String? = null,
)

@Serializable
private data class RewardLibrary(val rewards: List<LibraryReward> = emptyList())

/**
 * Native reward authoring for a Knight (SQUIRE-T-0074): create redeemable rewards (cost + Once /
 * Repeatable, optionally gated behind an achievement), import from the bundled starter library, and
 * archive — over the LAN api's `RequireKnight` `/admin/items` endpoints. Completes the authoring trio
 * (quests + achievements + rewards) on the phone. Styled to the "playful quest" theme.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
internal fun RewardAdminScreen(
    adapter: KnightApiAdapter,
    onBack: () -> Unit,
    // Screenshot/test seams (T-0070); all null in production → live fetch + bundled library.
    initialItems: List<ItemSummaryDto>? = null,
    libraryOverride: List<LibraryReward>? = null,
    gateOverride: List<Pair<Long, String>>? = null,
) {
    androidx.activity.compose.BackHandler { onBack() }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    var items by remember { mutableStateOf(initialItems ?: emptyList()) }
    var listError by remember { mutableStateOf<String?>(null) }
    var tick by remember { mutableStateOf(0) }

    // Gate options: current achievements (id → name). "None" is implicit (null gate).
    var gates by remember { mutableStateOf(gateOverride ?: emptyList()) }

    LaunchedEffect(tick) {
        if (initialItems != null) return@LaunchedEffect
        runCatching { adapter.listItems() }
            .onSuccess { items = it; listError = null }
            .onFailure { listError = "Couldn't load rewards — is the computer reachable?" }
        if (gateOverride == null) {
            runCatching { adapter.listAchievements() }
                .onSuccess { gates = it.filter { a -> a.active }.map { a -> a.id to a.name } }
        }
    }

    val library = libraryOverride ?: remember {
        runCatching {
            val text = context.assets.open("rewards-library.json").bufferedReader().use { it.readText() }
            Json { ignoreUnknownKeys = true }.decodeFromString(RewardLibrary.serializer(), text).rewards
        }.getOrDefault(emptyList())
    }

    // ── create-form state ──
    var name by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var cost by remember { mutableStateOf("10") }
    var availability by remember { mutableStateOf("Repeatable") } // Repeatable | Once
    var gate by remember { mutableStateOf<Long?>(null) }          // null = None
    var icon by remember { mutableStateOf("") }
    var formError by remember { mutableStateOf<String?>(null) }
    var composing by remember { mutableStateOf(false) } // the form is open (list-first screens)
    var editingId by remember { mutableStateOf<Long?>(null) }     // non-null = editing (upsert) — SQUIRE-T-0120

    fun resetForm() {
        name = ""; description = ""; cost = "10"; availability = "Repeatable"; gate = null; icon = ""; formError = null
        editingId = null
        composing = false
    }

    // Prefill the form from an existing reward and enter edit mode (SQUIRE-T-0120/0126).
    fun startEdit(r: ItemSummaryDto) {
        name = r.name
        description = r.description ?: ""
        cost = r.cost.toString()
        availability = if (r.availability == AvailabilityKind.Once) "Once" else "Repeatable"
        gate = r.gate
        icon = r.icon ?: ""
        editingId = r.id
        composing = true
        formError = null
    }

    fun create(req: CreateItemReq, onDone: () -> Unit = {}) {
        scope.launch {
            runCatching { adapter.createItem(req) }
                .onSuccess { tick++; onDone() }
                .onFailure { formError = "Couldn't save — check the fields and the connection." }
        }
    }

    fun submitForm() {
        if (name.isBlank()) { formError = "Add a name."; return }
        val c = cost.toLongOrNull()
        if (c == null || c < 1) { formError = "Cost must be at least 1 point."; return }
        formError = null
        create(
            CreateItemReq(
                availability = if (availability == "Once") AvailabilityKind.Once else AvailabilityKind.Repeatable,
                cost = c,
                name = name.trim(),
                description = description.trim().ifBlank { null },
                gate = gate,
                icon = icon.trim().ifBlank { null },
                id = editingId, // reuse id when editing (upsert) — SQUIRE-T-0120
            ),
        ) { resetForm() }
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("🎁 Rewards", style = MaterialTheme.typography.titleLarge) },
                navigationIcon = {
                    TextButton(onClick = onBack, colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.onPrimary)) { Text("Back") }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primary,
                    titleContentColor = MaterialTheme.colorScheme.onPrimary,
                ),
            )
        },
    ) { padding ->
        // Opening the form scrolls it into view (SQUIRE-T-0143): it sits above the list, so a parent who
        // has scrolled down and tapped Edit would otherwise see nothing happen — which is exactly the bug
        // the list-first change shipped with.
        val listState = androidx.compose.foundation.lazy.rememberLazyListState()
        LaunchedEffect(composing, editingId) { if (composing) listState.animateScrollToItem(0) }

        LazyColumn(
            state = listState,
            modifier = Modifier.fillMaxWidth().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            // List FIRST (SQUIRE-T-0134 follow-up): what is on the board is what you came to see. The
            // form opens on demand — "New reward" — or when editing an existing one.
            item {
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                    SectionTitle("Current rewards")
                    Spacer(Modifier.weight(1f))
                    if (!composing) FilledTonalButton(onClick = { composing = true }) { Text("New reward") }
                }
            }
            if (composing) {
                item { SectionTitle(if (editingId == null) "New reward" else "Edit reward") }
                item {
                    Card(
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                    ) {
                        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                            OutlinedTextField(value = name, onValueChange = { name = it }, label = { Text("Name") }, singleLine = true, modifier = Modifier.fillMaxWidth())
                            OutlinedTextField(value = description, onValueChange = { description = it }, label = { Text("Description (optional)") }, singleLine = true, modifier = Modifier.fillMaxWidth())
                            OutlinedTextField(value = cost, onValueChange = { cost = it.filter(Char::isDigit) }, label = { Text("Cost (points)") }, singleLine = true, modifier = Modifier.fillMaxWidth())

                            Text("Availability", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                ChoiceChip("Repeatable", availability == "Repeatable") { availability = "Repeatable" }
                                ChoiceChip("Once", availability == "Once") { availability = "Once" }
                            }

                            Text("Requires achievement (gate)", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                ChoiceChip("None", gate == null) { gate = null }
                                for ((id, label) in gates) {
                                    ChoiceChip(label, gate == id) { gate = id }
                                }
                            }

                            OutlinedTextField(value = icon, onValueChange = { icon = it }, label = { Text("Icon (emoji, optional)") }, singleLine = true, modifier = Modifier.fillMaxWidth())

                            formError?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                                Button(
                                    onClick = { submitForm() },
                                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.secondary, contentColor = MaterialTheme.colorScheme.onSecondary),
                                ) { Text(if (editingId == null) "Add reward" else "Save changes") }
                                if (editingId != null) {
                                    OutlinedButton(onClick = { resetForm() }) { Text("Cancel") }
                                }
                            }
                        }
                    }
                }

                if (library.isNotEmpty()) {
                    item { SectionTitle("Add from the starter library") }
                    items(library) { r ->
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Row(
                                Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 10.dp),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Medallion(null, r.name, reward = true)
                                Column(Modifier.weight(1f)) {
                                    Text(r.name, fontWeight = FontWeight.SemiBold)
                                    Text(libSummary(r), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                                GoldPill(r.cost.toInt())
                                OutlinedButton(onClick = { create(libToReq(r)) }) { Text("Import") }
                            }
                        }
                    }
                }
            }

            listError?.let { e -> item { Text(e, color = MaterialTheme.colorScheme.error) } }
            if (items.isEmpty() && listError == null) {
                item { Text("None yet — tap “New reward” or import from the library.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            items(items) { r ->
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                  Column {
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 10.dp),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Medallion(null, r.name, reward = true)
                        Column(Modifier.weight(1f)) {
                            Text(r.name + if (r.active) "" else " (archived)", fontWeight = FontWeight.SemiBold)
                            Text(r.summary, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        GoldPill(r.cost.toInt())
                    }
                    if (r.active) {
                        Row(Modifier.padding(start = 14.dp, end = 14.dp, bottom = 10.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                OutlinedButton(contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp), onClick = { startEdit(r) }) { Text("Edit") }
                                OutlinedButton(contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp), onClick = { scope.launch { runCatching { adapter.archiveItem(r.id) }; tick++ } }) { Text("Archive") }
                        }
                    }
                }
                }
            }        }
    }
}

private fun libToReq(r: LibraryReward): CreateItemReq = CreateItemReq(
    availability = if (r.availability == "Once") AvailabilityKind.Once else AvailabilityKind.Repeatable,
    cost = r.cost,
    name = r.name,
    description = r.description,
    gate = null,
    icon = null,
    id = null,
)

private fun libSummary(r: LibraryReward): String {
    val avail = if (r.availability == "Once") "once" else "repeatable"
    return "${r.cost} pts · $avail"
}
