package io.github.jacobbalane.veyl.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.os.Build
import android.os.SystemClock
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.jacobbalane.veyl.BuildConfig
import io.github.jacobbalane.veyl.R
import io.github.jacobbalane.veyl.capture.CaptureDebug
import io.github.jacobbalane.veyl.runtime.StreamLevel
import io.github.jacobbalane.veyl.runtime.VeylRuntime
import kotlinx.coroutines.delay

@Composable
fun DebugLabContent(
    runtime: VeylRuntime,
    onStart: (String) -> Unit,
    onStop: () -> Unit,
    onBack: () -> Unit,
    onChooseMusic: () -> Unit,
    onChooseFolder: () -> Unit,
) {
    val state by runtime.capture.collectAsStateWithLifecycle()
    val recording by CaptureDebug.recording.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val none = stringResource(R.string.none)
    val title = stringResource(R.string.lab_title)
    var confirm by remember { mutableStateOf(false) }
    var copied by remember { mutableStateOf(false) }
    var elapsed by remember { mutableLongStateOf(0L) }
    val active = state.state in listOf("starting", "running")
    LaunchedEffect(active, state.startedMs) {
        while (active) {
            elapsed = (SystemClock.elapsedRealtime() - state.startedMs) / 1000
            delay(1000)
        }
    }
    Text(stringResource(R.string.lab_title), style = MaterialTheme.typography.headlineLarge)
    Text(stringResource(R.string.lab_intro))
    Text(stringResource(R.string.lab_instructions))
    if (Build.VERSION.SDK_INT < 29) Text(stringResource(R.string.lab_unavailable))
    SourceControls(runtime, !active, onStart, onChooseMusic, onChooseFolder, R.string.lab_start)
    if (active) Button(onClick = onStop) { Text(stringResource(R.string.stop)) }
    if (state.source == "test_sound" && active) Text(stringResource(R.string.test_pause_others))
    if (state.source == "player" && active)
        PlayerControls(runtime) {
            io.github.jacobbalane.veyl.capture.ListeningService.player(context, it)
        }
    if (state.state == "error")
        Text(stringResource(R.string.lab_error), color = MaterialTheme.colorScheme.error)
    state.reason?.let { Text(it) }
    if (state.route == "external") Text(stringResource(R.string.not_speaker))
    if (state.micSilenced) Text(stringResource(R.string.mic_silenced))
    Text(stringResource(R.string.lab_elapsed, elapsed))
    state.info?.let { info ->
        Text(stringResource(R.string.lab_rate, info.rate))
        Text(stringResource(R.string.lab_source, info.micSource))
        Text(
            stringResource(
                R.string.lab_effects,
                info.effectsOff.joinToString().ifEmpty { none },
            ),
        )
    }
    Level(R.string.lab_reference, state.reference)
    Level(R.string.lab_microphone, state.microphone)
    Button(
        onClick = { confirm = true },
        enabled = state.state == "running" && recording != "recording",
    ) {
        Text(stringResource(R.string.lab_record))
    }
    Text(
        stringResource(
            when (recording) {
                "recording" -> R.string.lab_recording
                "saved" -> R.string.lab_saved
                "error" -> R.string.lab_error
                else -> R.string.lab_empty
            },
        ),
    )
    TextButton(onClick = { CaptureDebug.delete(context) }, enabled = recording != "recording") {
        Text(stringResource(R.string.lab_delete))
    }
    TextButton(
        onClick = {
            val info = state.info
            val diagnostics =
                "Veyl ${BuildConfig.VERSION_NAME} debug; Android ${Build.VERSION.RELEASE} (${Build.VERSION.SDK_INT}); ${Build.MANUFACTURER} ${Build.MODEL}\n" +
                    "state=${state.state}; source=${state.source}; reason=${state.reason}; elapsed_s=$elapsed; route=${state.route}; mic_silenced=${state.micSilenced}; hp_cutoff_hz=${runtime.settings.state.value.cutoff}\n" +
                    "rate=${info?.rate}; source=${info?.micSource}; channels=${info?.micChannels}; effects_off=${info?.effectsOff}; effects_on=${info?.effectsOn}\n" +
                    "ref_blocks=${state.reference.blocks}; ref_dbfs=${state.reference.rms}; ref_high_dbfs=${state.reference.highRms}\n" +
                    "mic_blocks=${state.microphone.blocks}; mic_dbfs=${state.microphone.rms}; mic_high_dbfs=${state.microphone.highRms}"
            context
                .getSystemService(ClipboardManager::class.java)
                .setPrimaryClip(
                    ClipData.newPlainText(title, diagnostics),
                )
            copied = true
        },
    ) {
        Text(stringResource(if (copied) R.string.lab_copied else R.string.lab_diagnostics))
    }
    TextButton(onClick = onBack) { Text(stringResource(R.string.back)) }
    if (confirm)
        AlertDialog(
            onDismissRequest = { confirm = false },
            title = { Text(stringResource(R.string.lab_record)) },
            text = { Text(stringResource(R.string.lab_warning)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirm = false
                        CaptureDebug.record()
                    },
                ) {
                    Text(stringResource(R.string.lab_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirm = false }) { Text(stringResource(R.string.cancel)) }
            },
        )
}

@Composable
private fun Level(title: Int, level: StreamLevel) {
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp)) {
            Text(stringResource(title), style = MaterialTheme.typography.titleMedium)
            Text(stringResource(R.string.lab_channels, level.channels))
            Text(stringResource(R.string.lab_blocks, level.blocks))
            Text(stringResource(R.string.lab_level, level.rms))
            Text(stringResource(R.string.lab_high, level.highRms))
        }
    }
}
