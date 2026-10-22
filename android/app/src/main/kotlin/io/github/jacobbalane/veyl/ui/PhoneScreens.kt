package io.github.jacobbalane.veyl.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.os.Build
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.jacobbalane.veyl.BuildConfig
import io.github.jacobbalane.veyl.R
import io.github.jacobbalane.veyl.runtime.VeylRuntime

@Composable
fun SourceControls(
    runtime: VeylRuntime,
    allowed: Boolean,
    onStart: (String) -> Unit,
    onChooseMusic: () -> Unit,
    onChooseFolder: () -> Unit,
    startLabel: Int = R.string.start_listening,
) {
    val settings by runtime.settings.state.collectAsStateWithLifecycle()
    val capture by runtime.capture.collectAsStateWithLifecycle()
    val active = capture.state == "running"
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for ((source, label) in
            listOf(
                "capture" to R.string.source_capture,
                "player" to R.string.source_player,
                "test_sound" to R.string.source_test,
            )) FilterChip(
            selected = settings.source == source,
            onClick = {
                runtime.settings.source(source)
                if (active && (source != "player" || settings.playerItems.isNotEmpty()))
                    onStart(source)
            },
            enabled =
                capture.state != "starting" && (source != "capture" || Build.VERSION.SDK_INT >= 29),
            label = { Text(stringResource(label)) },
        )
    }
    Text(
        stringResource(
            when (settings.source) {
                "player" -> R.string.hint_player
                "test_sound" -> R.string.hint_test
                else -> R.string.hint_capture
            },
        ),
    )
    if (Build.VERSION.SDK_INT < 29) Text(stringResource(R.string.capture_needs_10))
    if (settings.source == "player") {
        if (settings.playerItems.isEmpty()) Text(stringResource(R.string.no_music))
        TextButton(onClick = onChooseMusic, enabled = !active) {
            Text(stringResource(R.string.choose_music))
        }
        TextButton(onClick = onChooseFolder, enabled = !active) {
            Text(stringResource(R.string.choose_folder))
        }
    }
    if (!active)
        Button(
            onClick = { onStart(settings.source) },
            enabled =
                allowed &&
                    capture.state != "starting" &&
                    (settings.source != "player" || settings.playerItems.isNotEmpty()),
        ) {
            Text(stringResource(startLabel))
        }
}

@Composable
fun PlayerControls(runtime: VeylRuntime, command: (String) -> Unit) {
    val player by runtime.player.collectAsStateWithLifecycle()
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        TextButton(onClick = { command("previous") }, enabled = player.count > 1) {
            Text(stringResource(R.string.previous))
        }
        Button(onClick = { command(if (player.playing) "pause" else "play") }) {
            Text(stringResource(if (player.playing) R.string.pause else R.string.play))
        }
        TextButton(onClick = { command("next") }, enabled = player.count > 1) {
            Text(stringResource(R.string.next))
        }
        FilterChip(
            selected = player.shuffle,
            onClick = { command("shuffle") },
            label = { Text(stringResource(R.string.shuffle)) },
        )
    }
}

@Composable
fun SettingsContent(
    runtime: VeylRuntime,
    onCheck: () -> Unit,
    onBattery: () -> Unit,
    onAbout: () -> Unit,
    onBack: () -> Unit,
) {
    val settings by runtime.settings.state.collectAsStateWithLifecycle()
    var name by rememberSaveable { mutableStateOf(settings.phoneName) }
    var lower by remember { mutableStateOf<Int?>(null) }
    var copied by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val diagnosticLabel = stringResource(R.string.copy_diagnostics)
    Text(stringResource(R.string.settings), style = MaterialTheme.typography.headlineLarge)
    OutlinedTextField(
        value = name,
        onValueChange = {
            name = it.take(40)
            runtime.settings.name(name)
        },
        label = { Text(stringResource(R.string.phone_name)) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
    for ((cutoff, label) in
        listOf(
            8000 to R.string.privacy_8,
            7000 to R.string.privacy_7,
            6000 to R.string.privacy_6,
        )) {
        Row(
            Modifier.fillMaxWidth()
                .selectable(
                    selected = settings.cutoff == cutoff,
                    role = Role.RadioButton,
                    onClick = {
                        if (cutoff < 8000 && cutoff != settings.cutoff) lower = cutoff
                        else runtime.settings.cutoff(cutoff)
                    },
                ),
        ) {
            RadioButton(
                selected = settings.cutoff == cutoff,
                onClick = null,
            )
            Text(stringResource(label), Modifier.padding(top = 12.dp))
        }
    }
    SettingToggle(R.string.dim_screen, settings.dim, runtime.settings::dim)
    SettingToggle(R.string.pip_setting, settings.pip, runtime.settings::pip)
    TextButton(onClick = onBattery) { Text(stringResource(R.string.battery_setting)) }
    Text(stringResource(R.string.battery_detail))
    Button(onClick = onCheck) { Text(stringResource(R.string.run_check)) }
    TextButton(onClick = onAbout) { Text(stringResource(R.string.about)) }
    TextButton(
        onClick = {
            context
                .getSystemService(ClipboardManager::class.java)
                .setPrimaryClip(ClipData.newPlainText(diagnosticLabel, runtime.diagnostics()))
            copied = true
        },
    ) {
        Text(stringResource(if (copied) R.string.diagnostics_copied else R.string.copy_diagnostics))
    }
    TextButton(onClick = onBack) { Text(stringResource(R.string.back)) }
    if (lower != null)
        AlertDialog(
            onDismissRequest = { lower = null },
            text = { Text(stringResource(R.string.privacy_warning)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        runtime.settings.cutoff(lower!!)
                        lower = null
                    },
                ) {
                    Text(stringResource(R.string.continue_label))
                }
            },
            dismissButton = {
                TextButton(onClick = { lower = null }) { Text(stringResource(R.string.cancel)) }
            },
        )
}

@Composable
private fun SettingToggle(label: Int, value: Boolean, change: (Boolean) -> Unit) {
    Row(
        Modifier.fillMaxWidth()
            .heightIn(min = 48.dp)
            .toggleable(value = value, role = Role.Switch, onValueChange = change),
    ) {
        Text(stringResource(label), Modifier.weight(1f))
        Switch(checked = value, onCheckedChange = null)
    }
}

@Composable
fun SoundCheckContent(runtime: VeylRuntime, onContinue: () -> Unit, onStop: () -> Unit) {
    val check by runtime.check.collectAsStateWithLifecycle()
    if (check.state == "checking" || check.state == "idle") {
        Text(stringResource(R.string.checking), style = MaterialTheme.typography.headlineMedium)
        Text(stringResource(R.string.checking_detail))
        if (check.total > 0)
            LinearProgressIndicator(
                progress = { check.current.toFloat() / check.total },
                modifier = Modifier.fillMaxWidth(),
            )
        else LinearProgressIndicator(Modifier.fillMaxWidth())
        Button(onClick = onStop) { Text(stringResource(R.string.stop)) }
    } else {
        Text(
            stringResource(
                if (check.state == "error") R.string.capture_error_alert
                else if ((check.result?.meanCoherence ?: 0.0) < 0.2) R.string.check_low
                else R.string.check_done,
            ),
            style = MaterialTheme.typography.headlineMedium,
        )
        Button(onClick = onContinue) { Text(stringResource(R.string.continue_label)) }
    }
}

@Composable
fun PlacementContent(onBack: () -> Unit) {
    Text(stringResource(R.string.placement_title), style = MaterialTheme.typography.headlineLarge)
    for (text in
        listOf(
            R.string.placement_flat,
            R.string.placement_uncovered,
            R.string.placement_volume,
            R.string.placement_hand,
        )) Text(stringResource(text))
    TextButton(onClick = onBack) { Text(stringResource(R.string.back)) }
}

@Composable
fun AboutContent(open: (String) -> Unit, onLicenses: () -> Unit, onBack: () -> Unit) {
    Text(stringResource(R.string.about), style = MaterialTheme.typography.headlineLarge)
    Text(stringResource(R.string.version, BuildConfig.VERSION_NAME))
    Text(stringResource(R.string.license))
    TextButton(onClick = { open("https://jacobbalane.github.io/Veyl/") }) {
        Text(stringResource(R.string.website))
    }
    TextButton(onClick = { open("https://github.com/jacobbalane/Veyl") }) {
        Text(stringResource(R.string.source_code))
    }
    TextButton(onClick = onLicenses) { Text(stringResource(R.string.licenses)) }
    TextButton(onClick = onBack) { Text(stringResource(R.string.back)) }
}

@Composable
fun LicensesContent(onBack: () -> Unit) {
    val assets = LocalContext.current.assets
    val notices = remember {
        listOf("veyl.txt", "manrope.txt", "bouncycastle.txt", "androidx.txt").joinToString("\n\n") {
            assets.open("licenses/$it").bufferedReader().use { it.readText() }
        }
    }
    Text(stringResource(R.string.licenses), style = MaterialTheme.typography.headlineLarge)
    Text(notices, style = MaterialTheme.typography.bodySmall)
    TextButton(onClick = onBack) { Text(stringResource(R.string.back)) }
}
