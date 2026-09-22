package com.squire.app.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.squire.app.R

// ── Squire palette — the HERALDIC system (SQUIRE-I-0006 / SQUIRE-T-0132), shared 1:1 with the Keep's
//    CSS so phone + computer read as one product. One rule, from heraldry's rule of tincture:
//      · colour belongs to PEOPLE   (a squire's tincture marks what is theirs)
//      · gold belongs to REWARD     (the Gold* values below are for coins and coin amounts ONLY)
//      · everything else is quiet   (argent ground, white cards, midnight ink)
//    Red is for real errors. Declining a claim is "not yet", never an error. ──
private val Argent     = Color(0xFFEEF1F7) // app ground
private val Card       = Color(0xFFFFFFFF)
private val Wash       = Color(0xFFE6EBF5) // tinted disc behind content emoji; quiet fills
private val Ink        = Color(0xFF16203F) // midnight: text, bars, primary actions
private val Stone      = Color(0xFF5E6885) // secondary text
private val Line       = Color(0xFFDCE1EC)
private val Azure      = Color(0xFF2457C5) // interactive accent
private val AzureSoft  = Color(0xFFDEE8FB)
private val Vert       = Color(0xFF187A4F) // default squire tincture; "I did it" / earned
private val VertDeep   = Color(0xFF0F5C3A)
private val VertSoft   = Color(0xFFDCF0E6)
private val ErrorRed   = Color(0xFFB3261E)

// ── Public accents for hand-built components. Names predate the Heraldic palette and are kept so
//    call sites don't churn; the VALUES are what changed. ──
val SquireGold        = Color(0xFFB8860B) // coin rim / gold line-work
val SquireGoldLight   = Color(0xFFFFD978) // coin highlight
val SquireGoldFace    = Color(0xFFF6B93B) // coin face
val SquireGoldOn      = Color(0xFF7A4E00) // legible coin amount on a light ground
val SquireGoldSoft    = Color(0xFFFFF3D1) // coin-pill ground
val SquireRoyal       = Ink               // was royal purple: now the midnight ink
val SquireRoyalDeep   = Ink
val SquireGreen       = Vert
val SquireEpic        = Azure             // "streak heat" no longer borrows an orange
val SquireParchmentEdge = Line
val SquireParchmentTile = Wash
val SquireWash        = Wash
val SquireInk         = Ink
val SquireSeal        = Color(0xFFC8102E) // the Knight's wax seal — the MARK, not a button colour
val SquireCash        = Color(0xFF0B7A55)
val SquireCashSoft    = Color(0xFFDDF3EA)

private val SquireColors = lightColorScheme(
    primary = Ink,
    onPrimary = Color.White,
    primaryContainer = Wash,
    onPrimaryContainer = Ink,
    // NOT gold: `secondary*` feeds every tonal button, chip and nav indicator in Material 3, which
    // is how gold used to leak onto things that are not coins.
    secondary = Azure,
    onSecondary = Color.White,
    secondaryContainer = AzureSoft,
    onSecondaryContainer = Ink,
    tertiary = Vert,
    onTertiary = Color.White,
    tertiaryContainer = VertSoft,
    onTertiaryContainer = VertDeep,
    background = Argent,
    onBackground = Ink,
    surface = Card,
    onSurface = Ink,
    surfaceVariant = Wash,
    onSurfaceVariant = Stone,
    error = ErrorRed,
    onError = Color.White,
    errorContainer = Color(0xFFFCE8E6),
    onErrorContainer = Color(0xFF8C1D18),
    outline = Color(0xFFB9C0D0),
    outlineVariant = Line,
)

// Two bundled typefaces (SIL OFL 1.1; licences in assets/licenses), cut as static weights so they
// render identically on every API level and in Paparazzi:
//   · Grenze — a roman/blackletter hybrid: names, screen titles and every coin amount.
//   · Lexend — drawn to aid reading fluency (our readers are children): everything else.
val SquireDisplay = FontFamily(
    Font(R.font.grenze_semibold, FontWeight.SemiBold),
    Font(R.font.grenze_bold, FontWeight.Bold),
    Font(R.font.grenze_extrabold, FontWeight.ExtraBold),
)
val SquireBody = FontFamily(
    Font(R.font.lexend_light, FontWeight.Light),
    Font(R.font.lexend_regular, FontWeight.Normal),
    Font(R.font.lexend_medium, FontWeight.Medium),
    Font(R.font.lexend_semibold, FontWeight.SemiBold),
)

private val SquireType = Typography().run {
    fun TextStyle.body() = copy(fontFamily = SquireBody)
    copy(
        displayLarge = displayLarge.body(), displayMedium = displayMedium.body(), displaySmall = displaySmall.body(),
        headlineLarge = headlineLarge.copy(fontFamily = SquireDisplay, fontWeight = FontWeight.ExtraBold),
        headlineMedium = headlineMedium.copy(fontFamily = SquireDisplay, fontWeight = FontWeight.Bold),
        headlineSmall = headlineSmall.copy(fontFamily = SquireDisplay, fontWeight = FontWeight.Bold),
        titleLarge = titleLarge.copy(fontFamily = SquireDisplay, fontWeight = FontWeight.Bold, fontSize = 24.sp),
        // Below ~18sp Grenze's blackletter detail stops helping: smaller titles are Lexend.
        titleMedium = titleMedium.copy(fontFamily = SquireBody, fontWeight = FontWeight.Medium),
        titleSmall = titleSmall.copy(fontFamily = SquireBody, fontWeight = FontWeight.Medium),
        bodyLarge = bodyLarge.body(), bodyMedium = bodyMedium.body(), bodySmall = bodySmall.body(),
        labelLarge = labelLarge.copy(fontFamily = SquireBody, fontWeight = FontWeight.Medium),
        labelMedium = labelMedium.body(), labelSmall = labelSmall.body(),
    )
}

private val SquireShapes = Shapes(
    small = RoundedCornerShape(11.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(22.dp),
)

/** A squire's chosen colour (SQUIRE-T-0136): the tint their screens wear, a soft ground, and a
 *  deep text-on-soft. Names and hex mirror `contract::TINCTURES` and the Keep's swatches. */
data class Tincture(val name: String, val tint: Color, val soft: Color, val deep: Color)

val Tinctures: List<Tincture> = listOf(
    Tincture("gules",   Color(0xFFC22B3A), Color(0xFFFAE1E4), Color(0xFF7A1521)),
    Tincture("azure",   Color(0xFF2457C5), Color(0xFFDEE8FB), Color(0xFF153A8C)),
    Tincture("vert",    Color(0xFF187A4F), Color(0xFFDCF0E6), Color(0xFF0F5C3A)),
    Tincture("purpure", Color(0xFF6B3FA0), Color(0xFFECE3F7), Color(0xFF47276E)),
    Tincture("tenne",   Color(0xFFB8561B), Color(0xFFFBE6D8), Color(0xFF7A3609)),
    Tincture("sable",   Color(0xFF2E3440), Color(0xFFE4E7ED), Color(0xFF16203F)),
)

/** Resolve a stored tincture name (unknown/blank → the default, vert). */
fun tinctureOf(name: String?): Tincture =
    Tinctures.firstOrNull { it.name == name?.trim()?.lowercase() } ?: Tinctures[2]

/** The app's theme — wrap the whole UI so both the Squire and Knight surfaces inherit it.
 *
 *  [tincture] is the squire's colour: their top bar (`primary`) and their actions (`tertiary`)
 *  wear it. The Knight's surfaces pass nothing and stay on midnight ink. */
@Composable
fun SquireTheme(tincture: String? = null, content: @Composable () -> Unit) {
    val scheme = if (tincture == null) SquireColors else {
        val t = tinctureOf(tincture)
        SquireColors.copy(
            primary = t.tint, primaryContainer = t.soft, onPrimaryContainer = t.deep,
            tertiary = t.tint, tertiaryContainer = t.soft, onTertiaryContainer = t.deep,
        )
    }
    MaterialTheme(
        colorScheme = scheme,
        typography = SquireType,
        shapes = SquireShapes,
        content = content,
    )
}
