package io.github.jacobbalane.veyl.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.jacobbalane.veyl.R
import io.github.jacobbalane.veyl.runtime.PhoneMeter
import io.github.jacobbalane.veyl.runtime.VeylRuntime

@Composable
fun PhoneHomeContent(
    runtime: VeylRuntime,
    onStart: (String) -> Unit,
    chooseMusic: () -> Unit,
    chooseFolder: () -> Unit,
    placement: () -> Unit,
    settings: () -> Unit,
    allowNetwork: () -> Unit,
    appSettings: () -> Unit,
) {
    val link by runtime.link.state.collectAsStateWithLifecycle()
    val prefs by runtime.settings.state.collectAsStateWithLifecycle()
    Text(
        if (link.pcName == null) stringResource(R.string.no_pc_title)
        else stringResource(R.string.connected_to, link.pcName!!),
        style = MaterialTheme.typography.headlineLarge,
    )
    Text(stringResource(R.string.this_phone, prefs.phoneName))
    Text(stringResource(R.string.address, link.address ?: stringResource(R.string.not_available)))
    if (link.pcName == null) Text(stringResource(R.string.waiting_pc))
    if (!link.permission) {
        Text(stringResource(R.string.network_denied))
        Button(onClick = allowNetwork) { Text(stringResource(R.string.allow_network)) }
        TextButton(onClick = appSettings) { Text(stringResource(R.string.open_settings)) }
    } else if (link.address == null) Text(stringResource(R.string.same_network))
    if (link.pcName != null) SourceControls(runtime, true, onStart, chooseMusic, chooseFolder)
    link.pairedName?.let { Text(stringResource(R.string.paired_with, it)) }
    ConnectionError(link.error)
    TextButton(onClick = placement) { Text(stringResource(R.string.placement_link)) }
    TextButton(onClick = settings) { Text(stringResource(R.string.settings)) }
}

@Composable
fun PairingDialog(runtime: VeylRuntime) {
    val link by runtime.link.state.collectAsStateWithLifecycle()
    val prompt = link.pairing ?: return
    AlertDialog(
        onDismissRequest = { runtime.link.confirm(prompt.attempt, false) },
        title = { Text(stringResource(R.string.pair_title, prompt.pcName)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                Text(stringResource(R.string.pair_check))
                Text(prompt.code, style = MaterialTheme.typography.displayMedium)
                if (prompt.confirmed) Text(stringResource(R.string.waiting_pc))
            }
        },
        confirmButton = {
            TextButton(
                onClick = { runtime.link.confirm(prompt.attempt, true) },
                enabled = !prompt.confirmed,
            ) {
                Text(stringResource(R.string.codes_match))
            }
        },
        dismissButton = {
            TextButton(onClick = { runtime.link.confirm(prompt.attempt, false) }) {
                Text(stringResource(R.string.cancel))
            }
        },
    )
}

@Composable
fun ConnectionError(error: String?) {
    val text =
        when (error) {
            "pairing_failed" -> R.string.pair_failed
            "pair_store_read_failed" -> R.string.pair_store_read_failed
            "pair_store_write_failed" -> R.string.pair_store_write_failed
            else -> null
        }
    if (text != null) Text(stringResource(text), color = MaterialTheme.colorScheme.error)
}

@Composable
fun PhoneScore(meter: PhoneMeter?, small: Boolean = false) {
    if (meter == null) {
        Text(stringResource(R.string.no_pc_meter))
        return
    }
    Box(Modifier.size(if (small) 80.dp else 160.dp), contentAlignment = Alignment.Center) {
        CircularProgressIndicator(
            progress = { meter.score / 100f },
            modifier = Modifier.fillMaxSize(),
            strokeWidth = if (small) 6.dp else 10.dp,
        )
        Text(
            meter.score.toString(),
            style =
                if (small) MaterialTheme.typography.headlineLarge
                else MaterialTheme.typography.displayLarge,
        )
    }
    if (!small) {
        Text(
            stringResource(
                when (meter.label) {
                    "great" -> R.string.label_great
                    "good" -> R.string.label_good
                    "fair" -> R.string.label_fair
                    "poor" -> R.string.label_poor
                    else -> R.string.label_no_sound
                },
            ),
        )
        val tip =
            when (meter.tip) {
                "capture_off" -> R.string.tip_capture_off
                "not_speaker" -> R.string.not_speaker
                "capture_blocked" -> R.string.tip_capture_blocked
                "nothing_playing" -> R.string.tip_nothing_playing
                "mic_silenced" -> R.string.tip_mic_silenced
                "clipping" -> R.string.tip_clipping
                "volume_low" -> R.string.tip_volume_low
                "few_high_sounds" -> R.string.tip_few_high
                "gaps" -> R.string.tip_gaps
                "other_sounds" -> R.string.test_pause_others
                "unclear_echo" -> R.string.tip_unclear
                "noisy_room" -> R.string.tip_noisy
                "ready" -> R.string.tip_ready
                else -> null
            }
        if (tip != null) Text(stringResource(tip))
    }
}
