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
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.squire.app.ui.components.SectionTitle
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.sdk.model.CadenceKind
import com.squire.sdk.model.CompletionDto
import com.squire.sdk.model.CreateQuestReq
import com.squire.sdk.model.QuestSummaryDto
import com.squire.sdk.model.WeekdayDto
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

private val WEEKDAYS = listOf("Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun")

/** One entry from the bundled starter library (`assets/library.json`). `internal` so the screenshot
 * harness (SQUIRE-T-0070) can build sample library rows. */
@Serializable
internal data class LibraryQuest(
    val category: String,
    val title: String,
    val reward: Long,
    val cadence: String,
    val days: List<String> = emptyList(),
)

@Serializable
private data class QuestLibrary(val quests: List<LibraryQuest> = emptyList())

/**
 * Native quest authoring for a Knight (SQUIRE-T-0065): create/assign/schedule quests, import from the
 * bundled starter library, and archive existing ones — all over the LAN api's `RequireKnight`
 * `/admin/quests` endpoints (SQUIRE-T-0064). Styled to the "playful quest" theme.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
internal fun QuestAdminScreen(
    adapter: KnightApiAdapter,
    squires: List<Pair<Long, String>>,
    onBack: () -> Unit,
    // Screenshot/test seams (SQUIRE-T-0070); both null in production → live fetch + bundled library.
    initialQuests: List<QuestSummaryDto>? = null,
    libraryOverride: List<LibraryQuest>? = null,
) {
    androidx.activity.compose.BackHandler { onBack() }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    var quests by remember { mutableStateOf(initialQuests ?: emptyList()) }
    var listError by remember { mutableStateOf<String?>(null) }
    var tick by remember { mutableStateOf(0) }

    androidx.compose.runtime.LaunchedEffect(tick) {
        if (initialQuests != null) return@LaunchedEffect // injected (snapshot/test) — no network.
        runCatching { adapter.listQuests() }
            .onSuccess { quests = it; listError = null }
            .onFailure { listError = "Couldn't load quests — is the computer reachable?" }
    }

    val library = libraryOverride ?: remember {
        runCatching {
            val text = context.assets.open("library.json").bufferedReader().use { it.readText() }
            Json { ignoreUnknownKeys = true }.decodeFromString(QuestLibrary.serializer(), text).quests
        }.getOrDefault(emptyList())
    }

    // ── create-form state ──
    var title by remember { mutableStateOf("") }
    var reward by remember { mutableStateOf("5") }
    var category by remember { mutableStateOf("") }
    var cadence by remember { mutableStateOf("Daily") }
    val weekdays = remember { mutableStateListOf<String>() }
    var completion by remember { mutableStateOf("EachAssignee") }
    var assignAll by remember { mutableStateOf(true) }
    val chosenSquires = remember { mutableStateListOf<Long>() }
    var repeatDay by remember { mutableStateOf(false) }
    var autoApprove by remember { mutableStateOf(false) }
    var due by remember { mutableStateOf<Int?>(null) }
    var dueLabel by remember { mutableStateOf<String?>(null) }
    var showDatePicker by remember { mutableStateOf(false) }
    var formError by remember { mutableStateOf<String?>(null) }

    fun resetForm() {
        title = ""; reward = "5"; category = ""; cadence = "Daily"
        weekdays.clear(); completion = "EachAssignee"; assignAll = true
        chosenSquires.clear(); repeatDay = false; autoApprove = false
        due = null; dueLabel = null; formError = null
    }

    fun create(req: CreateQuestReq, onDone: () -> Unit = {}) {
        scope.launch {
            runCatching { adapter.createQuest(req) }
                .onSuccess { tick++; onDone() }
                .onFailure { formError = "Couldn't save — check the fields and the connection." }
        }
    }

    fun submitForm() {
        val rewardN = reward.toLongOrNull()
        if (title.isBlank() || rewardN == null) { formError = "Add a title and a reward."; return }
        if (cadence == "Weekly" && weekdays.isEmpty()) { formError = "Pick at least one weekday."; return }
        if (!assignAll && chosenSquires.isEmpty()) { formError = "Pick a squire, or choose All squires."; return }
        formError = null
        create(
            CreateQuestReq(
                assignAll = assignAll,
                autoApprove = autoApprove,
                cadence = CadenceKind.valueOf(cadence),
                completion = CompletionDto.valueOf(completion),
                repeatableWithinDay = repeatDay,
                reward = rewardN,
                title = title.trim(),
                category = category.trim().ifBlank { null },
                due = if (cadence == "OneOff") due else null,
                id = null,
                squires = if (assignAll) null else chosenSquires.toList(),
                weekdays = if (cadence == "Weekly") weekdays.map { WeekdayDto.valueOf(it) } else null,
            ),
        ) { resetForm() }
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("⚒ Manage Quests", style = MaterialTheme.typography.titleLarge) },
                navigationIcon = {
                    TextButton(
                        onClick = onBack,
                        colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.onPrimary),
                    ) { Text("Back") }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primary,
                    titleContentColor = MaterialTheme.colorScheme.onPrimary,
                ),
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxWidth().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            // ── New quest ──
            item { SectionTitle("New quest") }
            item {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                ) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        OutlinedTextField(value = title, onValueChange = { title = it }, label = { Text("Title") }, singleLine = true, modifier = Modifier.fillMaxWidth())
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                            OutlinedTextField(value = reward, onValueChange = { reward = it.filter(Char::isDigit) }, label = { Text("Reward ★") }, singleLine = true, modifier = Modifier.weight(1f))
                            OutlinedTextField(value = category, onValueChange = { category = it }, label = { Text("Category") }, singleLine = true, modifier = Modifier.weight(2f))
                        }

                        Text("How often", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            ChoiceChip("Every day", cadence == "Daily") { cadence = "Daily" }
                            ChoiceChip("Weekly", cadence == "Weekly") { cadence = "Weekly" }
                            ChoiceChip("One-time", cadence == "OneOff") { cadence = "OneOff" }
                        }
                        if (cadence == "Weekly") {
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                WEEKDAYS.forEach { d ->
                                    ChoiceChip(d, weekdays.contains(d)) {
                                        if (weekdays.contains(d)) weekdays.remove(d) else weekdays.add(d)
                                    }
                                }
                            }
                        }
                        if (cadence == "OneOff") {
                            OutlinedButton(onClick = { showDatePicker = true }) {
                                Text(dueLabel?.let { "Due: $it" } ?: "Set due date (optional)")
                            }
                        }

                        Text("Completion", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            ChoiceChip("Each does their own", completion == "EachAssignee") { completion = "EachAssignee" }
                            ChoiceChip("First to finish wins", completion == "Race") { completion = "Race" }
                        }

                        Text("Assign to", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            ChoiceChip("All squires", assignAll) { assignAll = true }
                            ChoiceChip("Specific", !assignAll) { assignAll = false }
                        }
                        if (!assignAll) {
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                squires.forEach { (id, name) ->
                                    ChoiceChip(name, chosenSquires.contains(id)) {
                                        if (chosenSquires.contains(id)) chosenSquires.remove(id) else chosenSquires.add(id)
                                    }
                                }
                            }
                        }

                        ToggleRow("Can be earned multiple times per day", repeatDay) { repeatDay = it }
                        ToggleRow("Auto-approve (skip review)", autoApprove) { autoApprove = it }

                        formError?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
                        Button(
                            onClick = { submitForm() },
                            colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.secondary, contentColor = MaterialTheme.colorScheme.onSecondary),
                        ) { Text("Add quest") }
                    }
                }
            }

            // ── Library ──
            if (library.isNotEmpty()) {
                item { SectionTitle("Add from the starter library") }
                items(library) { lib ->
                    Card(
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Row(
                            Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 10.dp),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(Modifier.weight(1f)) {
                                Text(lib.title, fontWeight = FontWeight.SemiBold)
                                Text(
                                    "${lib.category} · ${lib.reward} ★ · ${if (lib.cadence == "weekly") lib.days.joinToString("/") else "Daily"}",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            OutlinedButton(onClick = {
                                create(
                                    CreateQuestReq(
                                        assignAll = true,
                                        autoApprove = false,
                                        cadence = if (lib.cadence == "weekly") CadenceKind.Weekly else CadenceKind.Daily,
                                        completion = CompletionDto.EachAssignee,
                                        repeatableWithinDay = false,
                                        reward = lib.reward,
                                        title = lib.title,
                                        category = lib.category,
                                        due = null,
                                        id = null,
                                        squires = null,
                                        weekdays = if (lib.cadence == "weekly") lib.days.map { WeekdayDto.valueOf(it) } else null,
                                    ),
                                )
                            }) { Text("Import") }
                        }
                    }
                }
            }

            // ── Existing quests ──
            item { SectionTitle("Current quests") }
            listError?.let { e -> item { Text(e, color = MaterialTheme.colorScheme.error) } }
            if (quests.isEmpty() && listError == null) {
                item { Text("No quests yet — create one above or import from the library.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            items(quests) { q ->
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 10.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(Modifier.weight(1f)) {
                            Text(q.title + if (q.active) "" else " (archived)", fontWeight = FontWeight.SemiBold)
                            Text(
                                "${q.reward} ★ · ${q.cadenceLabel} · ${q.assignmentLabel}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        if (q.active) {
                            OutlinedButton(onClick = { scope.launch { runCatching { adapter.archiveQuest(q.id) }; tick++ } }) {
                                Text("Archive")
                            }
                        }
                    }
                }
            }
        }
    }

    if (showDatePicker) {
        val dpState = rememberDatePickerState()
        DatePickerDialog(
            onDismissRequest = { showDatePicker = false },
            confirmButton = {
                TextButton(onClick = {
                    dpState.selectedDateMillis?.let { ms ->
                        // The DatePicker returns UTC midnight millis; convert to the domain's
                        // Monday-aligned day-count (floor(unixDays) + 3), matching the server.
                        due = ((ms / 86_400_000L) + 3).toInt()
                        dueLabel = "day ${due}"
                    }
                    showDatePicker = false
                }) { Text("OK") }
            },
            dismissButton = { TextButton(onClick = { showDatePicker = false }) { Text("Cancel") } },
        ) { DatePicker(state = dpState) }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun ChoiceChip(label: String, selected: Boolean, onClick: () -> Unit) {
    FilterChip(
        selected = selected,
        onClick = onClick,
        label = { Text(label) },
        colors = FilterChipDefaults.filterChipColors(
            selectedContainerColor = MaterialTheme.colorScheme.primary,
            selectedLabelColor = MaterialTheme.colorScheme.onPrimary,
        ),
    )
}

@Composable
internal fun ToggleRow(label: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
        Switch(checked = checked, onCheckedChange = onChange)
    }
}
