package io.github.jacobbalane.veyl.ui

import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import io.github.jacobbalane.veyl.R
import io.github.jacobbalane.veyl.runtime.CaptureSnapshot

@Composable
fun ListeningContent(state: CaptureSnapshot, onStop: () -> Unit) {
    Text(stringResource(R.string.listening_channel), style = MaterialTheme.typography.headlineLarge)
    Text(stringResource(R.string.waiting_pc))
    Text(stringResource(R.string.no_pc_meter), style = MaterialTheme.typography.displaySmall)
    Text(stringResource(R.string.listening_reminder))
    if (state.route == "external")
        Text(stringResource(R.string.not_speaker), color = MaterialTheme.colorScheme.error)
    if (state.micSilenced)
        Text(stringResource(R.string.mic_silenced), color = MaterialTheme.colorScheme.error)
    if (state.source == "test_sound") Text(stringResource(R.string.test_pause_others))
    Button(onClick = onStop) { Text(stringResource(R.string.stop)) }
}
