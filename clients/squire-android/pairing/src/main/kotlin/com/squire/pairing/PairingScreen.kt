package com.squire.pairing

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import kotlinx.coroutines.launch

/**
 * First-run device pairing UI (ADR SQUIRE-A-0010 / SQUIRE-T-0046). Three ways to obtain a session:
 *  - **Scan QR** (ZXing) — the Keep's `squire://pair?...` QR; auto-exchanges on a successful scan.
 *  - **Enter manually** — host / port / household / code (also used to demo without a camera).
 *  - **Use demo creds** — a debug-only bypass ([demoLogin] non-null only in debug builds).
 *
 * "Discover" prefills host/port from NSD (best-effort). On success [onPaired] is called with the
 * stored-ready [Session]; the host app persists it and proceeds to its home screen.
 *
 * @param demoLogin debug-only shortcut producing a [Session] from baked demo creds; null in release.
 */
@Composable
fun PairingScreen(
    discovery: NsdDiscovery,
    demoLogin: (suspend () -> Session)?,
    onPaired: (Session) -> Unit,
) {
    val scope = rememberCoroutineScope()
    var host by remember { mutableStateOf("") }
    var port by remember { mutableStateOf("8080") }
    var household by remember { mutableStateOf("") }
    var code by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    fun pair(target: PairTarget) {
        busy = true
        error = null
        scope.launch {
            try {
                onPaired(PairingClient.pair(target))
            } catch (e: Exception) {
                error = "Pairing failed — check the code, host, and that the computer is reachable."
            } finally {
                busy = false
            }
        }
    }

    val scanLauncher = rememberLauncherForActivityResult(ScanContract()) { result ->
        val raw = result.contents
        if (raw != null) {
            val target = PairTarget.parse(raw)
            if (target == null) {
                error = "That QR isn't a Squire pairing code."
            } else {
                host = target.host
                port = target.port.toString()
                household = target.household
                code = target.code
                pair(target)
            }
        }
    }

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Pair this device", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            "On the computer, open the Keep → Pair a device, pick this member, and scan the QR — " +
                "or type the code below.",
            style = MaterialTheme.typography.bodyMedium,
        )

        Button(
            onClick = { scanLauncher.launch(ScanOptions().setOrientationLocked(false).setBeepEnabled(false)) },
            enabled = !busy,
            modifier = Modifier.fillMaxWidth(),
        ) { Text("Scan QR") }

        Text("Or enter it manually", style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.Medium)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
            OutlinedTextField(
                value = host, onValueChange = { host = it }, label = { Text("Host") },
                singleLine = true, modifier = Modifier.weight(2f),
            )
            OutlinedTextField(
                value = port, onValueChange = { port = it.filter(Char::isDigit) }, label = { Text("Port") },
                singleLine = true, modifier = Modifier.weight(1f),
            )
        }
        OutlinedTextField(
            value = household, onValueChange = { household = it }, label = { Text("Household") },
            singleLine = true, modifier = Modifier.fillMaxWidth(),
        )
        OutlinedTextField(
            value = code, onValueChange = { code = it }, label = { Text("Pairing code") },
            singleLine = true, modifier = Modifier.fillMaxWidth(),
        )

        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
            OutlinedButton(
                onClick = {
                    scope.launch {
                        discovery.discover()?.let { (h, p) -> host = h; port = p.toString() }
                            ?: run { error = "No computer found on the network — enter the host manually." }
                    }
                },
                enabled = !busy,
            ) { Text("Discover") }
            Button(
                onClick = {
                    val p = port.toIntOrNull()
                    if (host.isBlank() || p == null || household.isBlank() || code.isBlank()) {
                        error = "Fill in host, port, household, and code."
                    } else {
                        pair(PairTarget(host.trim(), p, household.trim(), code.trim()))
                    }
                },
                enabled = !busy,
                modifier = Modifier.weight(1f),
            ) { Text("Pair") }
        }

        if (demoLogin != null) {
            OutlinedButton(
                onClick = {
                    busy = true; error = null
                    scope.launch {
                        try {
                            onPaired(demoLogin())
                        } catch (e: Exception) {
                            error = "Demo login failed — is squire-home running?"
                        } finally {
                            busy = false
                        }
                    }
                },
                enabled = !busy,
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Use demo creds (debug)") }
        }

        if (busy) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                CircularProgressIndicator()
                Text("Pairing…")
            }
        }
        error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    }
}
