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
import com.squire.sdk.model.AchBasisKind
import com.squire.sdk.model.AchCriterionKind
import com.squire.sdk.model.AchScopeKind
import com.squire.sdk.model.AchievementSummaryDto
import com.squire.sdk.model.CreateAchievementReq
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/** One entry from the bundled achievement starter library (`assets/achievements-library.json`).
 * `internal` so the screenshot harness (T-0070) can build sample rows. */
@Serializable
internal data class LibraryAch(
    val name: String,
    val criterion: String,            // "streak" | "total" | "points"
    val scope: String = "any",        // "category" | "any"
    val category: String? = null,
    val length: Long? = null,
    val basis: String? = null,        // "CalendarDays" | "ScheduledOccurrences"
    val count: Long? = null,
    val total: Long? = null,
    val bonus: Long,
)

@Serializable
private data class AchLibrary(val achievements: List<LibraryAch> = emptyList())

/**
 * Native achievement authoring for a Knight (SQUIRE-T-0072): create milestones (streak / total /
 * points, scoped Any or a category), import from the bundled starter library, and archive — over the
 * LAN api's `RequireKnight` `/admin/achievements` endpoints. Scope: Any chore, a category, or one
 * specific quest (SQUIRE-T-0094 #4). Styled to the "playful quest" theme.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
internal fun AchievementAdminScreen(
    adapter: KnightApiAdapter,
    onBack: () -> Unit,
    // Screenshot/test seams (T-0070); both null in production → live fetch + bundled library.
    initialAchievements: List<AchievementSummaryDto>? = null,
    libraryOverride: List<LibraryAch>? = null,
) {
    androidx.activity.compose.BackHandler { onBack() }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    var achievements by remember { mutableStateOf(initialAchievements ?: emptyList()) }
    var listError by remember { mutableStateOf<String?>(null) }
    var tick by remember { mutableStateOf(0) }

    LaunchedEffect(tick) {
        if (initialAchievements != null) return@LaunchedEffect
        runCatching { adapter.listAchievements() }
            .onSuccess { achievements = it; listError = null }
            .onFailure { listError = "Couldn't load achievements — is the computer reachable?" }
    }

    val library = libraryOverride ?: remember {
        runCatching {
            val text = context.assets.open("achievements-library.json").bufferedReader().use { it.readText() }
            Json { ignoreUnknownKeys = true }.decodeFromString(AchLibrary.serializer(), text).achievements
        }.getOrDefault(emptyList())
    }

    // ── create-form state ──
    var name by remember { mutableStateOf("") }
    var criterion by remember { mutableStateOf("Streak") } // Streak | TotalCompletions | PointsEarned
    var achScope by remember { mutableStateOf("Any") }     // Any | Category | Quest
    var category by remember { mutableStateOf("") }
    var scopeQuestId by remember { mutableStateOf<Long?>(null) }
    var quests by remember { mutableStateOf<List<Pair<Long, String>>>(emptyList()) }
    var length by remember { mutableStateOf("7") }
    var basis by remember { mutableStateOf("CalendarDays") }
    var count by remember { mutableStateOf("10") }
    var total by remember { mutableStateOf("100") }
    var bonus by remember { mutableStateOf("10") }
    var formError by remember { mutableStateOf<String?>(null) }
    var composing by remember { mutableStateOf(false) } // the form is open (list-first screens)
    var editingId by remember { mutableStateOf<Long?>(null) } // non-null = editing (upsert) — SQUIRE-T-0120

    // Quests to pick from when scoping an achievement/streak to one specific quest (SQUIRE-T-0094 #4).
    LaunchedEffect(Unit) {
        if (initialAchievements != null) return@LaunchedEffect
        runCatching { adapter.listQuests() }
            .onSuccess { list -> quests = list.filter { it.active }.map { it.id to it.title } }
    }

    fun resetForm() {
        name = ""; criterion = "Streak"; achScope = "Any"; category = ""; scopeQuestId = null
        length = "7"; basis = "CalendarDays"; count = "10"; total = "100"; bonus = "10"; formError = null
        editingId = null
        composing = false
    }

    // Prefill the form from an existing achievement and enter edit mode (SQUIRE-T-0120/0126).
    fun startEdit(a: AchievementSummaryDto) {
        name = a.name
        criterion = when (a.criterion) {
            AchCriterionKind.TotalCompletions -> "TotalCompletions"
            AchCriterionKind.PointsEarned -> "PointsEarned"
            else -> "Streak"
        }
        achScope = when (a.scope) {
            AchScopeKind.Category -> "Category"
            AchScopeKind.Quest -> "Quest"
            else -> "Any"
        }
        category = a.scopeCategory ?: ""
        scopeQuestId = a.scopeQuest
        length = (a.length ?: 7).toString()
        basis = a.basis?.name ?: "CalendarDays"
        count = (a.count ?: 10).toString()
        total = (a.total ?: 100).toString()
        bonus = a.bonus.toString()
        editingId = a.id
        composing = true
        formError = null
    }

    fun create(req: CreateAchievementReq, onDone: () -> Unit = {}) {
        scope.launch {
            runCatching { adapter.createAchievement(req) }
                .onSuccess { tick++; onDone() }
                .onFailure { formError = "Couldn't save — check the fields and the connection." }
        }
    }

    fun submitForm() {
        val bonusN = bonus.toLongOrNull() ?: 0
        if (name.isBlank()) { formError = "Add a name."; return }
        val usesScope = criterion != "PointsEarned"
        if (usesScope && achScope == "Category" && category.isBlank()) { formError = "Enter a category for the scope."; return }
        if (usesScope && achScope == "Quest" && scopeQuestId == null) { formError = "Pick a quest for the scope."; return }
        val req = when (criterion) {
            "PointsEarned" -> {
                val t = total.toLongOrNull(); if (t == null || t < 1) { formError = "Points must be at least 1."; return }
                baseReq(name, AchCriterionKind.PointsEarned, AchScopeKind.Any, null, bonusN, total = t)
            }
            "TotalCompletions" -> {
                val c = count.toLongOrNull(); if (c == null || c < 1) { formError = "Count must be at least 1."; return }
                baseReq(name, AchCriterionKind.TotalCompletions, scopeKind(achScope), category, bonusN, count = c, questId = scopeQuestId)
            }
            else -> {
                val l = length.toLongOrNull(); if (l == null || l < 1) { formError = "Length must be at least 1."; return }
                baseReq(name, AchCriterionKind.Streak, scopeKind(achScope), category, bonusN, length = l, basis = AchBasisKind.valueOf(basis), questId = scopeQuestId)
            }
        }
        formError = null
        create(req.copy(id = editingId)) { resetForm() } // reuse id when editing (upsert) — SQUIRE-T-0120
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("🏅 Achievements", style = MaterialTheme.typography.titleLarge) },
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
        LazyColumn(
            modifier = Modifier.fillMaxWidth().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            // List FIRST (SQUIRE-T-0134 follow-up): what is on the board is what you came to see. The
            // form opens on demand — "New achievement" — or when editing an existing one.
            item {
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                    SectionTitle("Current achievements")
                    Spacer(Modifier.weight(1f))
                    if (!composing) FilledTonalButton(onClick = { composing = true }) { Text("New achievement") }
                }
            }
            listError?.let { e -> item { Text(e, color = MaterialTheme.colorScheme.error) } }
            if (achievements.isEmpty() && listError == null) {
                item { Text("None yet — tap “New achievement” or import from the library.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            items(achievements) { a ->
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
                        Medallion("🛡")
                        Column(Modifier.weight(1f)) {
                            Text(a.name + if (a.active) "" else " (archived)", fontWeight = FontWeight.SemiBold)
                            Text(a.summary, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        if (a.bonus > 0) GoldPill(a.bonus.toInt())
                    }
                    if (a.active) {
                        Row(Modifier.padding(start = 14.dp, end = 14.dp, bottom = 10.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                OutlinedButton(contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp), onClick = { startEdit(a) }) { Text("Edit") }
                                OutlinedButton(contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp), onClick = { scope.launch { runCatching { adapter.archiveAchievement(a.id) }; tick++ } }) { Text("Archive") }
                        }
                    }
                }
                }
            }
            if (composing) {
                item { SectionTitle(if (editingId == null) "New achievement" else "Edit achievement") }
                item {
                    Card(
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                    ) {
                        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                            OutlinedTextField(value = name, onValueChange = { name = it }, label = { Text("Name") }, singleLine = true, modifier = Modifier.fillMaxWidth())

                            Text("Earn it by", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                ChoiceChip("A streak", criterion == "Streak") { criterion = "Streak" }
                                ChoiceChip("Completing N", criterion == "TotalCompletions") { criterion = "TotalCompletions" }
                                ChoiceChip("Earning points", criterion == "PointsEarned") { criterion = "PointsEarned" }
                            }

                            if (criterion != "PointsEarned") {
                                Text("Across", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
                                FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    ChoiceChip("Any chore", achScope == "Any") { achScope = "Any" }
                                    ChoiceChip("A category", achScope == "Category") { achScope = "Category" }
                                    ChoiceChip("A specific quest", achScope == "Quest") { achScope = "Quest" }
                                }
                                if (achScope == "Category") {
                                    OutlinedTextField(value = category, onValueChange = { category = it }, label = { Text("Category (e.g. Bedroom)") }, singleLine = true, modifier = Modifier.fillMaxWidth())
                                }
                                if (achScope == "Quest") {
                                    if (quests.isEmpty()) {
                                        Text("No quests yet — create a quest first, then scope to it.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    } else {
                                        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                            quests.forEach { (id, title) ->
                                                ChoiceChip(title, scopeQuestId == id) { scopeQuestId = id }
                                            }
                                        }
                                    }
                                }
                            }

                            when (criterion) {
                                "PointsEarned" -> OutlinedTextField(value = total, onValueChange = { total = it.filter(Char::isDigit) }, label = { Text("Points to earn") }, singleLine = true, modifier = Modifier.fillMaxWidth())
                                "TotalCompletions" -> OutlinedTextField(value = count, onValueChange = { count = it.filter(Char::isDigit) }, label = { Text("How many completions") }, singleLine = true, modifier = Modifier.fillMaxWidth())
                                else -> {
                                    OutlinedTextField(value = length, onValueChange = { length = it.filter(Char::isDigit) }, label = { Text("Streak length (days)") }, singleLine = true, modifier = Modifier.fillMaxWidth())
                                    Text("Counted by", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
                                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                        ChoiceChip("Calendar days", basis == "CalendarDays") { basis = "CalendarDays" }
                                        ChoiceChip("Scheduled days", basis == "ScheduledOccurrences") { basis = "ScheduledOccurrences" }
                                    }
                                }
                            }

                            OutlinedTextField(value = bonus, onValueChange = { bonus = it.filter(Char::isDigit) }, label = { Text("Bonus coins (0 = unlock only)") }, singleLine = true, modifier = Modifier.fillMaxWidth())

                            formError?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                                Button(
                                    onClick = { submitForm() },
                                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.secondary, contentColor = MaterialTheme.colorScheme.onSecondary),
                                ) { Text(if (editingId == null) "Add achievement" else "Save changes") }
                                if (editingId != null) {
                                    OutlinedButton(onClick = { resetForm() }) { Text("Cancel") }
                                }
                            }
                        }
                    }
                }

                if (library.isNotEmpty()) {
                    item { SectionTitle("Add from the starter library") }
                    items(library) { a ->
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
                                    Text(a.name, fontWeight = FontWeight.SemiBold)
                                    Text(libSummary(a), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                                OutlinedButton(onClick = { create(libToReq(a)) }) { Text("Import") }
                            }
                        }
                    }
                }
            }
        }
    }
}

private fun scopeKind(s: String) = when (s) {
    "Category" -> AchScopeKind.Category
    "Quest" -> AchScopeKind.Quest
    else -> AchScopeKind.Any
}

private fun baseReq(
    name: String,
    criterion: AchCriterionKind,
    scope: AchScopeKind,
    category: String?,
    bonus: Long,
    length: Long? = null,
    basis: AchBasisKind? = null,
    count: Long? = null,
    total: Long? = null,
    questId: Long? = null,
) = CreateAchievementReq(
    bonus = bonus,
    criterion = criterion,
    name = name.trim(),
    scope = scope,
    basis = basis,
    count = count,
    id = null,
    length = length,
    scopeCategory = if (scope == AchScopeKind.Category) category?.trim() else null,
    scopeQuest = if (scope == AchScopeKind.Quest) questId else null,
    total = total,
)

private fun libToReq(a: LibraryAch): CreateAchievementReq {
    val crit = when (a.criterion) {
        "points" -> AchCriterionKind.PointsEarned
        "total" -> AchCriterionKind.TotalCompletions
        else -> AchCriterionKind.Streak
    }
    val scope = if (a.scope == "category") AchScopeKind.Category else AchScopeKind.Any
    return baseReq(
        name = a.name, criterion = crit, scope = scope, category = a.category, bonus = a.bonus,
        length = a.length, basis = a.basis?.let { AchBasisKind.valueOf(it) }, count = a.count, total = a.total,
    )
}

private fun libSummary(a: LibraryAch): String {
    val scope = if (a.scope == "category") (a.category ?: "any") else "any"
    return when (a.criterion) {
        "points" -> "${a.total} points · +${a.bonus} coins"
        "total" -> "${a.count} completions · $scope · +${a.bonus} coins"
        else -> "${a.length}-day streak · $scope · +${a.bonus} coins"
    }
}
