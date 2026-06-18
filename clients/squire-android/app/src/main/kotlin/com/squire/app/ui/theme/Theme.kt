package com.squire.app.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// ── "Playful quest" brand palette (SQUIRE-T-0056): parchment + royal indigo + heraldic gold ──
private val ParchmentBg = Color(0xFFF4EAD3)
private val ParchmentCard = Color(0xFFFFFDF6)
private val Royal = Color(0xFF4B3F8F)        // primary — royal indigo/purple
private val RoyalDeep = Color(0xFF2E2657)
private val Gold = Color(0xFFB8860B)         // points / accents (dark goldenrod — legible)
private val GoldContainer = Color(0xFFF1DEA6)
private val GoldOnContainer = Color(0xFF5A4300)
private val Herald = Color(0xFF2E7D52)       // "done"/success green
private val Slate = Color(0xFF2B2533)        // body text
private val Crimson = Color(0xFF9E2B25)      // reject / error

/** Heraldic gold, exposed for accents that need the raw brand color. */
val SquireGold = Gold

private val SquireColors = lightColorScheme(
    primary = Royal,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFE6E1F6),
    onPrimaryContainer = RoyalDeep,
    secondary = Gold,
    onSecondary = Color.White,
    secondaryContainer = GoldContainer,
    onSecondaryContainer = GoldOnContainer,
    tertiary = Herald,
    onTertiary = Color.White,
    tertiaryContainer = Color(0xFFCDEBD7),
    onTertiaryContainer = Color(0xFF0F3D27),
    background = ParchmentBg,
    onBackground = Slate,
    surface = ParchmentCard,
    onSurface = Slate,
    surfaceVariant = Color(0xFFEADFC4),
    onSurfaceVariant = Color(0xFF5B5340),
    error = Crimson,
    onError = Color.White,
    errorContainer = Color(0xFFF6D9D6),
    onErrorContainer = Color(0xFF5A1714),
    outline = Color(0xFFC7B998),
    outlineVariant = Color(0xFFDDD0B2),
)

// Display headers use a serif (built-in Noto Serif) for the old-world feel; body stays sans.
private val Display = FontFamily.Serif

private val SquireType = Typography().run {
    copy(
        headlineMedium = headlineMedium.copy(fontFamily = Display, fontWeight = FontWeight.Bold),
        headlineSmall = headlineSmall.copy(fontFamily = Display, fontWeight = FontWeight.Bold),
        titleLarge = titleLarge.copy(fontFamily = Display, fontWeight = FontWeight.Bold, fontSize = 22.sp),
        titleMedium = titleMedium.copy(fontFamily = Display, fontWeight = FontWeight.SemiBold),
    )
}

private val SquireShapes = Shapes(
    small = RoundedCornerShape(10.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(22.dp),
)

/** The app's theme — wrap the whole UI so both the Squire and Knight surfaces inherit it. */
@Composable
fun SquireTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = SquireColors,
        typography = SquireType,
        shapes = SquireShapes,
        content = content,
    )
}
