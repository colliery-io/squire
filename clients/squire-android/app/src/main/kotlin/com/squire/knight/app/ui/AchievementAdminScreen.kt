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
 * LAN api's `RequireKnight` `/admin/achievements` endpoints. (Quest-scoped achievements stay on the
 * Keep for now.) Styled to the "playful quest" theme.
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
    var achScope by remember { mutableStateOf("Any") }     // Any | Category
    var category by remember { mutableStateOf("") }
    var length by remember { mutableStateOf("7") }
    var basis by remember { mutableStateOf("CalendarDays") }
    var count by remember { mutableStateOf("10") }
    var total by remember { mutableStateOf("100") }
    var bonus by remember { mutableStateOf("10") }
    var formError by remember { mutableStateOf<String?>(null) }

    fun resetForm() {
        name = ""; criterion = "Streak"; achScope = "Any"; category = ""
        length = "7"; basis = "CalendarDays"; count = "10"; total = "100"; bonus = "10"; formError = null
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
        val req = when (criterion) {
            "PointsEarned" -> {
                val t = total.toLongOrNull(); if (t == null || t < 1) { formError = "Points must be at least 1."; return }
                baseReq(name, AchCriterionKind.PointsEarned, AchScopeKind.Any, null, bonusN, total = t)
            }
            "TotalCompletions" -> {
                val c = count.toLongOrNull(); if (c == null || c < 1) { formError = "Count must be at least 1."; return }
                baseReq(name, AchCriterionKind.TotalCompletions, scopeKind(achScope), category, bonusN, count = c)
            }
            else -> {
                val l = length.toLongOrNull(); if (l == null || l < 1) { formError = "Length must be at least 1."; return }
                baseReq(name, AchCriterionKind.Streak, scopeKind(achScope), category, bonusN, length = l, basis = AchBasisKind.valueOf(basis))
            }
        }
        formError = null
        create(req) { resetForm() }
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
            item { SectionTitle("New achievement") }
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
                            }
                            if (achScope == "Category") {
                                OutlinedTextField(value = category, onValueChange = { category = it }, label = { Text("Category (e.g. Bedroom)") }, singleLine = true, modifier = Modifier.fillMaxWidth())
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
                        Button(
                            onClick = { submitForm() },
                            colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.secondary, contentColor = MaterialTheme.colorScheme.onSecondary),
                        ) { Text("Add achievement") }
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

            item { SectionTitle("Current achievements") }
            listError?.let { e -> item { Text(e, color = MaterialTheme.colorScheme.error) } }
            if (achievements.isEmpty() && listError == null) {
                item { Text("None yet — create one above or import from the library.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            items(achievements) { a ->
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
                        Medallion("🛡")
                        Column(Modifier.weight(1f)) {
                            Text(a.name + if (a.active) "" else " (archived)", fontWeight = FontWeight.SemiBold)
                            Text(a.summary, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        if (a.bonus > 0) GoldPill(a.bonus.toInt())
                        if (a.active) {
                            OutlinedButton(onClick = { scope.launch { runCatching { adapter.archiveAchievement(a.id) }; tick++ } }) { Text("Archive") }
                        }
                    }
                }
            }
        }
    }
}

private fun scopeKind(s: String) = if (s == "Category") AchScopeKind.Category else AchScopeKind.Any

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
    scopeQuest = null,
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
