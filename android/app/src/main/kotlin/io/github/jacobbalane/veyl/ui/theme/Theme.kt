package io.github.jacobbalane.veyl.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.ExperimentalTextApi
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import io.github.jacobbalane.veyl.R

private val dark = darkColorScheme(
    background = Color(0xFF0B0F14), surface = Color(0xFF121821),
    primary = Color(0xFF4FD8C4), onPrimary = Color(0xFF0B0F14),
    onBackground = Color(0xFFE8EEF5), onSurface = Color(0xFFE8EEF5),
    onSurfaceVariant = Color(0xFF94A3B8), outline = Color(0xFF1E2833),
)
private val light = lightColorScheme(
    background = Color(0xFFF7F9FB), surface = Color.White,
    primary = Color(0xFF0E9C8A), onPrimary = Color(0xFF0B0F14),
    onBackground = Color(0xFF0B0F14), onSurface = Color(0xFF0B0F14),
    onSurfaceVariant = Color(0xFF5B6B7C), outline = Color(0xFFE2E8F0),
)

@OptIn(ExperimentalTextApi::class)
private val manrope = FontFamily(
    *listOf(400, 500, 600, 700, 800).map { weight ->
        Font(R.font.manrope_variable, FontWeight(weight), variationSettings = FontVariation.Settings(FontVariation.weight(weight)))
    }.toTypedArray(),
)
private val defaults = Typography()
private val typography = Typography(
    displaySmall = defaults.displaySmall.copy(fontFamily = manrope, fontWeight = FontWeight.ExtraBold),
    headlineMedium = defaults.headlineMedium.copy(fontFamily = manrope, fontWeight = FontWeight.Bold),
    titleMedium = defaults.titleMedium.copy(fontFamily = manrope),
    bodyLarge = defaults.bodyLarge.copy(fontFamily = manrope),
    bodyMedium = defaults.bodyMedium.copy(fontFamily = manrope),
    labelLarge = defaults.labelLarge.copy(fontFamily = manrope),
)

@Composable
fun VeylTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = if (isSystemInDarkTheme()) dark else light, typography = typography, content = content)
}
