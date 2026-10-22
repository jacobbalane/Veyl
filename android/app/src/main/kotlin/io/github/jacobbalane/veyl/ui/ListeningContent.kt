package io.github.jacobbalane.veyl.ui

import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import io.github.jacobbalane.veyl.R
import io.github.jacobbalane.veyl.runtime.CaptureSnapshot
import io.github.jacobbalane.veyl.runtime.PhoneLinkState

@Composable
fun ListeningContent(state: CaptureSnapshot, link: PhoneLinkState, onStop: () -> Unit) {
    Text(stringResource(R.string.listening_channel), style = MaterialTheme.typography.headlineLarge)
    Text(
        if (link.pcName == null) stringResource(R.string.waiting_pc)
        else stringResource(R.string.connected_to, link.pcName),
    )
    PhoneScore(link.meter)
    Text(stringResource(R.string.listening_reminder))
    if (state.route == "external")
        Text(stringResource(R.string.not_speaker), color = MaterialTheme.colorScheme.error)
    if (state.micSilenced)
        Text(stringResource(R.string.mic_silenced), color = MaterialTheme.colorScheme.error)
    if (state.source == "test_sound") Text(stringResource(R.string.test_pause_others))
    Button(onClick = onStop) { Text(stringResource(R.string.stop)) }
}
