package com.squire.knight.app.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
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
import androidx.compose.material3.IconButton
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.DropdownMenu
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Spacer
import androidx.compose.material3.FilledTonalButton
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.squire.app.ui.components.Medallion
import com.squire.app.ui.components.SectionTitle
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.pairing.PairQr
import com.squire.pairing.PairTarget
import com.squire.sdk.model.MemberSummaryDto
import com.squire.sdk.model.Role
import kotlinx.coroutines.launch

/** A minted pairing code plus its rendered QR, shown in the pair dialog (SQUIRE-T-0075). */
internal data class PairInvite(
    val memberName: String,
    val code: String,
    val host: String,
    val port: Int,
    val household: String,
    val qr: androidx.compose.ui.graphics.ImageBitmap?,
)

/**
 * Native household member administration for a Knight (SQUIRE-T-0075): list members, add a Knight or
 * Squire, de/reactivate (archive-not-delete, never your own account), and **pair a member's device**
 * by minting a one-time code and showing it as text + a QR the new device scans. Parity with the
 * Keep's Members + Pair tabs. Styled to the "playful quest" theme.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
internal fun MemberAdminScreen(
    adapter: KnightApiAdapter,
    selfUser: Long,
    host: String,
    port: Int,
    household: String,
    onBack: () -> Unit,
    // Screenshot/test seam (T-0070): non-null skips the live fetch.
    initialMembers: List<MemberSummaryDto>? = null,
) {
    androidx.activity.compose.BackHandler { onBack() }
    val scope = rememberCoroutineScope()

    var composing by remember { mutableStateOf(false) } // the add form is open (list-first)
    var members by remember { mutableStateOf(initialMembers ?: emptyList()) }
    var listError by remember { mutableStateOf<String?>(null) }
    var tick by remember { mutableStateOf(0) }

    LaunchedEffect(tick) {
        if (initialMembers != null) return@LaunchedEffect
        runCatching { adapter.listMembers() }
            .onSuccess { members = it; listError = null }
            .onFailure { listError = "Couldn't load members — is the computer reachable?" }
    }

    // ── add-member form state ──
    var name by remember { mutableStateOf("") }
    var role by remember { mutableStateOf("Squire") } // Squire | Knight
    var secret by remember { mutableStateOf("") }
    var formError by remember { mutableStateOf<String?>(null) }

    // ── pair dialog ──
    var invite by remember { mutableStateOf<PairInvite?>(null) }

    // ── rename dialog (SQUIRE-T-0120/0126) ──
    var renaming by remember { mutableStateOf<MemberSummaryDto?>(null) }
    var renameText by remember { mutableStateOf("") }

    fun submitAdd() {
        if (name.isBlank()) { formError = "Add a name."; return }
        if (secret.isBlank()) { formError = "Set an initial secret."; return }
        formError = null
        scope.launch {
            runCatching { adapter.addMember(name.trim(), if (role == "Knight") Role.Knight else Role.Squire, secret) }
                .onSuccess { name = ""; secret = ""; role = "Squire"; tick++ }
                .onFailure { formError = "Couldn't add that member — check the connection." }
        }
    }

    fun pair(member: MemberSummaryDto) {
        scope.launch {
            runCatching { adapter.mintPairCode(member.user) }
                .onSuccess { resp ->
                    val target = PairTarget(host = host, port = port, household = household, code = resp.code)
                    val bmp = PairQr.bitmap(PairQr.payload(target))?.asImageBitmap()
                    invite = PairInvite(member.displayName, resp.code, host, port, household, bmp)
                }
                .onFailure { listError = "Couldn't mint a pairing code — is the computer reachable?" }
        }
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("👪 Members", style = MaterialTheme.typography.titleLarge) },
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
            // form opens on demand — "Add a member" — or when editing an existing one.
            item {
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                    SectionTitle("Household")
                    Spacer(Modifier.weight(1f))
                    if (!composing) FilledTonalButton(onClick = { composing = true }) { Text("Add a member") }
                }
            }
            listError?.let { e -> item { Text(e, color = MaterialTheme.colorScheme.error) } }
            items(members) { m ->
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
                        Medallion(m.displayName.take(1).uppercase())
                        Column(Modifier.weight(1f)) {
                            Text(m.displayName + if (m.user == selfUser) " (you)" else "", fontWeight = FontWeight.SemiBold)
                            Text(m.role.value + if (m.active) "" else " · inactive", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        // Rare actions behind a menu (SQUIRE-T-0134 follow-up): three outlined buttons per
                        // member crushed the name column on a phone.
                        var menuOpen by remember { mutableStateOf(false) }
                        Box {
                            IconButton(onClick = { menuOpen = true }) {
                                Text("⋮", style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                            DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                                if (m.active) DropdownMenuItem(text = { Text("Pair a phone") }, onClick = { menuOpen = false; pair(m) })
                                // Rename in place (keeps id, role, pairing) — SQUIRE-T-0120/0126.
                                DropdownMenuItem(text = { Text("Rename") }, onClick = { menuOpen = false; renaming = m; renameText = m.displayName })
                                // Never offer to deactivate your own account (no self-lockout).
                                if (m.user != selfUser) {
                                    DropdownMenuItem(
                                        text = { Text(if (m.active) "Deactivate" else "Reactivate") },
                                        onClick = { menuOpen = false; scope.launch { runCatching { adapter.setMemberActive(m.user, !m.active) }; tick++ } },
                                    )
                                }
                            }
                        }
                    }
                }
            }
            if (composing) {
                item { SectionTitle("Add a member") }
                item {
                    Card(
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                    ) {
                        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                            OutlinedTextField(value = name, onValueChange = { name = it }, label = { Text("Name") }, singleLine = true, modifier = Modifier.fillMaxWidth())

                            Text("Role", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                ChoiceChip("Squire (child)", role == "Squire") { role = "Squire" }
                                ChoiceChip("Knight (parent)", role == "Knight") { role = "Knight" }
                            }

                            OutlinedTextField(value = secret, onValueChange = { secret = it }, label = { Text("Initial secret (fallback login)") }, singleLine = true, modifier = Modifier.fillMaxWidth())

                            formError?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
                            Button(
                                onClick = { submitAdd() },
                                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.secondary, contentColor = MaterialTheme.colorScheme.onSecondary),
                            ) { Text("Add member") }
                        }
                    }
                }
            }
        }
    }

    invite?.let { inv ->
        AlertDialog(
            onDismissRequest = { invite = null },
            confirmButton = { TextButton(onClick = { invite = null }) { Text("Done") } },
            title = { Text("Pair ${inv.memberName}'s device") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text("Scan this from the Squire/Knight app, or enter the details manually.", style = MaterialTheme.typography.bodySmall)
                    inv.qr?.let { Image(bitmap = it, contentDescription = "Pairing QR", modifier = Modifier.size(220.dp)) }
                    Text("Code: ${inv.code}", style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.SemiBold)
                    Text("Server: ${inv.host}:${inv.port}", style = MaterialTheme.typography.bodySmall)
                    Text("Household: ${inv.household}", style = MaterialTheme.typography.bodySmall)
                    Text("Single use · expires in ~30 min.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            },
        )
    }

    renaming?.let { m ->
        AlertDialog(
            onDismissRequest = { renaming = null },
            confirmButton = {
                TextButton(
                    enabled = renameText.isNotBlank() && renameText.trim() != m.displayName,
                    onClick = {
                        val next = renameText.trim()
                        scope.launch { runCatching { adapter.renameMember(m.user, next) }; tick++ }
                        renaming = null
                    },
                ) { Text("Save") }
            },
            dismissButton = { TextButton(onClick = { renaming = null }) { Text("Cancel") } },
            title = { Text("Rename ${m.displayName}") },
            text = {
                OutlinedTextField(
                    value = renameText,
                    onValueChange = { renameText = it },
                    label = { Text("Name") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
            },
        )
    }
}
