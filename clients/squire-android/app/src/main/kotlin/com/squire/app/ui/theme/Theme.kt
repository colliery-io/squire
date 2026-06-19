package com.squire.app.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// ── Squire brand palette (redesign 2026) — exact values from style-guide/color-palette.md,
//    shared 1:1 with the Keep desktop CSS so phone + computer read as one product. ──
private val ParchmentBg   = Color(0xFFF2E6C8) // app background
private val ParchmentCard = Color(0xFFFFFEFA) // card surface
private val Royal         = Color(0xFF5A3D8E) // primary — royal purple
private val RoyalDeep     = Color(0xFF402A67)
private val RoyalContainer = Color(0xFFE6E1F6)
private val GoldText      = Color(0xFFA9781F) // legible gold for text/icons on parchment
private val GoldContainer = Color(0xFFF3E3B3) // soft gold pill background
private val GoldOnContainer = Color(0xFF5A4300)
private val Herald        = Color(0xFF5D8A42) // success / "done" green
private val HeraldDeep    = Color(0xFF42642E)
private val GreenContainer = Color(0xFFE4EFD8)
private val Ink           = Color(0xFF3C2D21) // body text
private val Stone         = Color(0xFF6E6450) // secondary text
private val Hearth        = Color(0xFFB55443) // warning / reject
private val Line          = Color(0xFFE2D2AC)

// ── Public brand accents, for the redesigned components (medallion, coin pill, accents). ──
val SquireGold        = GoldText
val SquireGoldLight   = Color(0xFFF1C15C)
val SquireGoldOn      = GoldOnContainer
val SquireRoyal       = Royal
val SquireRoyalDeep   = RoyalDeep
val SquireGreen       = Herald
val SquireEpic        = Color(0xFFE08B2D) // high-value / streak heat
val SquireParchmentEdge = Color(0xFFDCC9A0)
val SquireParchmentTile = Color(0xFFFBF1D8)

private val SquireColors = lightColorScheme(
    primary = Royal,
    onPrimary = Color.White,
    primaryContainer = RoyalContainer,
    onPrimaryContainer = RoyalDeep,
    secondary = GoldText,
    onSecondary = Color.White,
    secondaryContainer = GoldContainer,
    onSecondaryContainer = GoldOnContainer,
    tertiary = Herald,
    onTertiary = Color.White,
    tertiaryContainer = GreenContainer,
    onTertiaryContainer = HeraldDeep,
    background = ParchmentBg,
    onBackground = Ink,
    surface = ParchmentCard,
    onSurface = Ink,
    surfaceVariant = Color(0xFFEADFC4),
    onSurfaceVariant = Stone,
    error = Hearth,
    onError = Color.White,
    errorContainer = Color(0xFFF3DED9),
    onErrorContainer = Color(0xFF8A3E32),
    outline = Color(0xFFC7B998),
    outlineVariant = Line,
)

// Display headers use a serif for the adventure-journal feel; body stays sans.
// To match the mockups exactly, drop a Bitter font resource into res/font/ and set:
//   private val Display = FontFamily(Font(R.font.bitter_bold, FontWeight.Bold), …)
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
    small = RoundedCornerShape(11.dp),
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
