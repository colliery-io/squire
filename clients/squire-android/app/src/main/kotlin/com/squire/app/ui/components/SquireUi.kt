package com.squire.app.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.squire.app.ui.theme.SquireSeal
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.runtime.remember
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.squire.app.ui.theme.SquireCash
import com.squire.app.ui.theme.SquireCashSoft
import com.squire.app.ui.theme.SquireDisplay
import com.squire.app.ui.theme.SquireGold
import com.squire.app.ui.theme.SquireGoldFace
import com.squire.app.ui.theme.SquireGoldSoft
import com.squire.app.ui.theme.SquireWash
import com.squire.app.ui.theme.SquireGoldLight
import com.squire.app.ui.theme.SquireGoldOn
import com.squire.app.ui.theme.SquireParchmentEdge
import com.squire.app.ui.theme.SquireParchmentTile

/**
 * A gold **coin pill** — a minted disc + amount, the system's currency token. [large] for the hero
 * balance in the app bar. Signature is unchanged from the original `GoldPill(amount, large)`.
 */
@Composable
fun GoldPill(amount: Int, large: Boolean = false, ghost: Boolean = false) {
    // `ghost` (SQUIRE-T-0133): coins that are CLAIMED but not yet sealed — the same pill, drained of
    // gold, so the child sees exactly what a grown-up's seal will turn to gold.
    Surface(
        shape = RoundedCornerShape(50),
        color = if (ghost) MaterialTheme.colorScheme.surfaceVariant else SquireGoldSoft,
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(if (large) 7.dp else 5.dp),
            modifier = Modifier.padding(horizontal = if (large) 14.dp else 9.dp, vertical = if (large) 7.dp else 5.dp),
        ) {
            Coin(size = if (large) 20.dp else 16.dp, ghost = ghost)
            Text(
                "$amount",
                color = if (ghost) MaterialTheme.colorScheme.onSurfaceVariant else SquireGoldOn,
                fontWeight = FontWeight.Bold,
                fontFamily = SquireDisplay,
                fontSize = if (large) 22.sp else 17.sp,
            )
        }
    }
}

/** Real-money "$ owed" pill (SQUIRE-T-0099) — visually distinct from the gold coin (green, banknote
 *  feel) so dollars never read as spendable coins. */
@Composable
fun CashPill(amount: Long, large: Boolean = true) {
    Surface(
        shape = RoundedCornerShape(50),
        color = SquireCashSoft,
    ) {
        Text(
            "$$amount",
            color = SquireCash,
            fontWeight = FontWeight.Bold,
            fontFamily = SquireDisplay,
            fontSize = if (large) 22.sp else 17.sp,
            modifier = Modifier.padding(horizontal = if (large) 14.dp else 9.dp, vertical = if (large) 7.dp else 5.dp),
        )
    }
}

/** The minted gold coin used inside coin pills and medallions — a plain disc with an inner ring
 *  (no star), so the single in-app currency reads unambiguously as "coins" (SQUIRE-T-0094). */
@Composable
private fun Coin(size: androidx.compose.ui.unit.Dp, ghost: Boolean = false) {
    val rim = if (ghost) MaterialTheme.colorScheme.outline else SquireGold
    androidx.compose.foundation.layout.Box(
        modifier = Modifier
            .size(size)
            .clip(CircleShape)
            .then(
                if (ghost) Modifier.background(MaterialTheme.colorScheme.surface)
                else Modifier.background(Brush.radialGradient(listOf(SquireGoldLight, SquireGoldFace))),
            )
            .border(1.5.dp, rim, CircleShape),
        contentAlignment = Alignment.Center,
    ) {
        androidx.compose.foundation.layout.Box(
            modifier = Modifier
                .size(size * 0.6f)
                .clip(CircleShape)
                .border(1.dp, rim.copy(alpha = 0.45f), CircleShape),
        )
    }
}

/** The Knight's wax seal (SQUIRE-T-0133): the mark that lands on a quest when a grown-up approves
 *  it. Seal-red, tilted, with a Grenze check — the one place the seal colour appears on the child's
 *  screen. Scales in when it first appears so approval is a moment, not a state change. */
@Composable
fun WaxSeal(size: androidx.compose.ui.unit.Dp = 40.dp, stamp: Boolean = false) {
    // `stamp`: play the landing animation. Only when an approval ARRIVES while the child is looking
    // (the caller saw this quest waiting a moment ago) — a quest that was already sealed when the
    // screen opened just sits there, sealed.
    val scale = remember { androidx.compose.animation.core.Animatable(if (stamp) 2.2f else 1f) }
    LaunchedEffect(stamp) {
        if (stamp) scale.animateTo(1f, androidx.compose.animation.core.spring(dampingRatio = 0.45f, stiffness = 900f))
    }
    androidx.compose.foundation.layout.Box(
        modifier = Modifier
            .size(size)
            .graphicsLayer { scaleX = scale.value; scaleY = scale.value; rotationZ = -9f }
            .clip(CircleShape)
            .background(SquireSeal)
            .border(3.dp, SquireSeal.copy(alpha = 0.45f), CircleShape),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            "✓",
            color = Color.White,
            fontFamily = SquireDisplay,
            fontWeight = FontWeight.ExtraBold,
            fontSize = (size.value * 0.5f).sp,
        )
    }
}

/** Where the seal will land: an empty dashed ring, so "waiting" has a shape (SQUIRE-T-0133). */
@Composable
fun SealSlot(size: androidx.compose.ui.unit.Dp = 40.dp) {
    val ring = MaterialTheme.colorScheme.outlineVariant
    androidx.compose.foundation.Canvas(modifier = Modifier.size(size)) {
        drawCircle(
            color = ring,
            style = androidx.compose.ui.graphics.drawscope.Stroke(
                width = 2.dp.toPx(),
                pathEffect = androidx.compose.ui.graphics.PathEffect.dashPathEffect(floatArrayOf(6f, 6f)),
            ),
        )
    }
}

/**
 * A **medallion** — the plain tinted disc behind a quest/reward's own emoji (content, not chrome). Pass the stored
 * [icon] (emoji) or let it infer one from [label]; set [reward] to use the treasure icon table.
 */
@Composable
fun Medallion(icon: String?, label: String = "", reward: Boolean = false, modifier: Modifier = Modifier) {
    val glyph = icon ?: inferEmoji(label, reward)
    androidx.compose.foundation.layout.Box(
        modifier = modifier
            .size(44.dp)
            .clip(CircleShape)
            .background(SquireWash),
        contentAlignment = Alignment.Center,
    ) {
        Text(glyph, fontSize = 22.sp)
    }
}

/** Keyword → emoji inference, mirroring the Keep's `keep.js` map (icons are posted as null today). */
private fun inferEmoji(label: String, reward: Boolean): String {
    val t = label.lowercase()
    fun has(vararg ks: String) = ks.any { t.contains(it) }
    return if (reward) when {
        has("ice cream", "sundae", "gelato") -> "🍦"
        has("movie", "film", "cinema") -> "🎬"
        has("screen", "game", "video", "tv", "console") -> "🎮"
        has("park", "outing", "trip", "zoo") -> "🌳"
        has("pizza", "dinner", "treat", "candy", "snack", "dessert") -> "🍕"
        has("book", "read", "story", "comic") -> "📖"
        has("money", "allowance", "cash", "coin") -> "💰"
        has("toy", "lego", "figure") -> "🧸"
        else -> "🎁"
    } else when {
        has("bed", "wake", "morning") -> "🛏"
        has("tidy", "room", "clean", "vacuum", "dust") -> "🧹"
        has("dog", "walk", "pet", "biscuit", "cat", "feed") -> "🐕"
        has("piano", "music", "practice", "guitar", "violin") -> "🎹"
        has("homework", "study", "read", "school", "book") -> "📚"
        has("table", "dish", "kitchen", "wash", "plate") -> "🍽"
        has("trash", "recycl", "bin", "garbage", "compost") -> "♻️"
        has("laundry", "fold") -> "🧺"
        has("tooth", "brush", "teeth") -> "🪥"
        has("water", "plant", "garden", "flower") -> "🪴"
        else -> "📜"
    }
}

/** A small status chip (e.g. "Pending review", "Done today", "Locked"). */
@Composable
fun StatusChip(text: String, container: Color, content: Color) {
    Surface(color = container, shape = RoundedCornerShape(50)) {
        Text(
            text,
            color = content,
            style = MaterialTheme.typography.labelMedium,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
        )
    }
}

/** A section heading in the display serif, royal-colored. */
@Composable
fun SectionTitle(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.titleMedium,
        color = MaterialTheme.colorScheme.primary,
        fontWeight = FontWeight.Bold,
    )
}

/** A streak progress as filled/empty dots, e.g. ●●●○○○○. Filled dots use brand gold. */
@Composable
fun ProgressDots(current: Int, target: Int) {
    Row(horizontalArrangement = Arrangement.spacedBy(5.dp), verticalAlignment = Alignment.CenterVertically) {
        val total = target.coerceIn(1, 10)
        repeat(total) { i ->
            Dot(filled = i < current)
        }
    }
}

@Composable
fun Dot(filled: Boolean) {
    androidx.compose.foundation.layout.Box(
        modifier = Modifier
            .size(11.dp)
            .clip(CircleShape)
            .background(if (filled) MaterialTheme.colorScheme.tertiary else MaterialTheme.colorScheme.outlineVariant),
    )
}

/** A soft full-width banner (offline / update / info). */
@Composable
fun Banner(text: String, container: Color, content: Color, trailing: @Composable (() -> Unit)? = null) {
    Surface(color = container, shape = RoundedCornerShape(14.dp)) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 11.dp),
        ) {
            Text(text, color = content, fontWeight = FontWeight.SemiBold, modifier = Modifier.weight(1f))
            trailing?.invoke()
        }
    }
}
