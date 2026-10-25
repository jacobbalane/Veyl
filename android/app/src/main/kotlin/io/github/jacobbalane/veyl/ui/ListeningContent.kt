package io.github.jacobbalane.veyl.ui

import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.res.stringResource
import kotlinx.coroutines.delay
import io.github.jacobbalane.veyl.R
import io.github.jacobbalane.veyl.runtime.CaptureSnapshot
import io.github.jacobbalane.veyl.runtime.PhoneLinkState

@Composable
fun ListeningContent(
    state: CaptureSnapshot,
    link: PhoneLinkState,
    onHelp: () -> Unit,
    onStop: () -> Unit,
) {
    Text(stringResource(R.string.listening_channel), style = MaterialTheme.typography.headlineLarge)
    Text(
        if (link.pcName == null) stringResource(R.string.waiting_pc)
        else stringResource(R.string.connected_to, link.pcName),
    )
    PhoneScore(link.meter)
    // A short flash for each poke the PC saw; the space stays reserved so nothing jumps.
    var flash by remember { mutableStateOf(false) }
    LaunchedEffect(link.pokes) {
        if (link.pokes > 0) {
            flash = true
            delay(600)
            flash = false
        }
    }
    Text(
        stringResource(R.string.poke_seen),
        style = MaterialTheme.typography.titleLarge,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.alpha(if (flash) 1f else 0f),
    )
    Text(stringResource(R.string.listening_reminder))
    if (state.route == "external")
        Text(stringResource(R.string.not_speaker), color = MaterialTheme.colorScheme.error)
    if (state.micSilenced)
        Text(stringResource(R.string.mic_silenced), color = MaterialTheme.colorScheme.error)
    if (state.source == "test_sound") Text(stringResource(R.string.test_pause_others))
    TextButton(onClick = onHelp) { Text(stringResource(R.string.troubleshooting_link)) }
    Button(onClick = onStop) { Text(stringResource(R.string.stop)) }
}
