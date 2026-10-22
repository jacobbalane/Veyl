package io.github.jacobbalane.veyl.ui

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.Image
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import io.github.jacobbalane.veyl.BuildConfig
import io.github.jacobbalane.veyl.R
import io.github.jacobbalane.veyl.ui.theme.VeylTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            VeylTheme {
                var started by rememberSaveable { mutableStateOf(false) }
                Scaffold { padding ->
                    Column(
                        modifier = Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(28.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center,
                    ) {
                        Image(painterResource(R.drawable.ic_launcher_foreground), contentDescription = null, modifier = Modifier.size(140.dp))
                        Text(stringResource(R.string.app_name), style = MaterialTheme.typography.displaySmall)
                        Spacer(Modifier.height(16.dp))
                        Text(stringResource(if (started) R.string.no_pc_title else R.string.tagline), style = MaterialTheme.typography.headlineMedium, textAlign = TextAlign.Center)
                        Spacer(Modifier.height(16.dp))
                        Text(stringResource(if (started) R.string.waiting_pc else R.string.welcome_description), textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Spacer(Modifier.height(28.dp))
                        Card {
                            Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                Text(stringResource(R.string.foundation_title), style = MaterialTheme.typography.titleMedium)
                                Text(stringResource(R.string.foundation_detail), style = MaterialTheme.typography.bodyMedium)
                            }
                        }
                        Spacer(Modifier.height(28.dp))
                        if (!started) Button(onClick = { started = true }) { Text(stringResource(R.string.get_started)) }
                        Spacer(Modifier.height(28.dp))
                        Text(stringResource(R.string.version, BuildConfig.VERSION_NAME), style = MaterialTheme.typography.bodyMedium)
                        Text(stringResource(R.string.license), style = MaterialTheme.typography.bodyMedium)
                    }
                }
            }
        }
    }
}
