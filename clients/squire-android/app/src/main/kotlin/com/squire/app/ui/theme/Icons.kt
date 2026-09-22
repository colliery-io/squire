package com.squire.app.ui.theme

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathFillType
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.path
import androidx.compose.ui.unit.dp

/**
 * Drawn icons for the app's chrome (SQUIRE-T-0137): the bottom navigation and section headers.
 * Emoji stay where they are CONTENT — a quest's or reward's own picture — and leave the chrome,
 * where they rendered faded and differed from phone to phone. Same 24-unit grid and 1.8 stroke as
 * the Keep's inline SVGs, so both surfaces read as one set. Tinted by the caller via `tint`.
 */
object SquireIcons {
    private fun stroked(name: String, vararg paths: String): ImageVector =
        ImageVector.Builder(name = name, defaultWidth = 24.dp, defaultHeight = 24.dp, viewportWidth = 24f, viewportHeight = 24f)
            .apply {
                for (d in paths) path(
                    fill = null, stroke = SolidColor(Color.Black), strokeLineWidth = 1.8f,
                    strokeLineCap = StrokeCap.Round, strokeLineJoin = StrokeJoin.Round, pathFillType = PathFillType.NonZero,
                ) { PathParser.parse(d, this) }
            }
            .build()

    /** A pennant — today's board. */
    val Today: ImageVector by lazy { stroked("Today", "M5 4 H19 V20 L12 16 L5 20 Z") }
    /** A treasure chest — rewards. */
    val Rewards: ImageVector by lazy { stroked("Rewards", "M4 9 H20 V20 H4 Z", "M4 9 L6 4 H18 L20 9", "M12 9 V20") }
    /** A shield — me, my arms. */
    val Me: ImageVector by lazy { stroked("Me", "M12 3 L19 6 V12 C19 17 16 20 12 21 C8 20 5 17 5 12 V6 Z") }
    /** A journal — activity / history. */
    val Journal: ImageVector by lazy { stroked("Journal", "M6 4 H18 V20 H6 Z", "M9 9 H15", "M9 13 H15") }
    /** A check in a ring — review / the seal. */
    val Review: ImageVector by lazy { stroked("Review", "M12 4 A8 8 0 1 1 11.99 4", "M8.5 12.5 L11 15 L15.5 10") }
    /** Crossed tools — manage. */
    val Manage: ImageVector by lazy { stroked("Manage", "M5 19 L14 10", "M10 5 L19 14", "M14 10 L17 7 A2 2 0 0 0 17 4 L15 6", "M5 19 A2 2 0 0 0 8 19") }
}

/** A tiny SVG path-data parser (M/L/H/V/C/A/Z, absolute) — enough for the icons above without a
 *  dependency. Not general: no relative commands, no S/Q/T. */
private object PathParser {
    fun parse(d: String, p: androidx.compose.ui.graphics.vector.PathBuilder) {
        val tokens = Regex("[MLHVCAZ]|-?\\d*\\.?\\d+").findAll(d).map { it.value }.toList()
        var i = 0
        fun num() = tokens[i++].toFloat()
        while (i < tokens.size) {
            when (tokens[i++]) {
                "M" -> p.moveTo(num(), num())
                "L" -> p.lineTo(num(), num())
                "H" -> p.horizontalLineTo(num())
                "V" -> p.verticalLineTo(num())
                "C" -> p.curveTo(num(), num(), num(), num(), num(), num())
                "A" -> { val rx = num(); val ry = num(); val rot = num(); val large = num() != 0f; val sweep = num() != 0f; p.arcTo(rx, ry, rot, large, sweep, num(), num()) }
                "Z" -> p.close()
            }
        }
    }
}
