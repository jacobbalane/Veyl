package io.github.jacobbalane.veyl.ui

import androidx.compose.runtime.Composable
import io.github.jacobbalane.veyl.runtime.VeylRuntime

@Composable
@Suppress("UNUSED_PARAMETER")
fun DebugLabContent(
    runtime: VeylRuntime,
    onStart: () -> Unit,
    onStop: () -> Unit,
    onBack: () -> Unit,
) {}
