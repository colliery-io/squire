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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.squire.app.ui.theme.SquireGold
import com.squire.app.ui.theme.SquireGoldLight
import com.squire.app.ui.theme.SquireGoldOn
import com.squire.app.ui.theme.SquireParchmentEdge
import com.squire.app.ui.theme.SquireParchmentTile

/**
 * A gold **coin pill** — a minted disc + amount, the system's currency token. [large] for the hero
 * balance in the app bar. Signature is unchanged from the original `GoldPill(amount, large)`.
 */
@Composable
fun GoldPill(amount: Int, large: Boolean = false) {
    Surface(
        shape = RoundedCornerShape(50),
        color = Color(0xFFF3E3B3),
        border = BorderStroke(1.dp, SquireGold.copy(alpha = .55f)),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(if (large) 7.dp else 5.dp),
            modifier = Modifier.padding(horizontal = if (large) 14.dp else 9.dp, vertical = if (large) 7.dp else 5.dp),
        ) {
            Coin(size = if (large) 20.dp else 16.dp)
            Text(
                "$amount",
                color = SquireGoldOn,
                fontWeight = FontWeight.Bold,
                fontFamily = FontFamily.Serif,
                fontSize = if (large) 20.sp else 15.sp,
            )
        }
    }
}

/** The minted gold disc used inside coin pills and medallions. */
@Composable
private fun Coin(size: androidx.compose.ui.unit.Dp) {
    androidx.compose.foundation.layout.Box(
        modifier = Modifier
            .size(size)
            .clip(CircleShape)
            .background(Brush.radialGradient(listOf(SquireGoldLight, SquireGold)))
            .border(1.5.dp, Color(0xFFA9781F), CircleShape),
        contentAlignment = Alignment.Center,
    ) {
        Text("★", color = Color(0xFF7A5A12), fontSize = (size.value * 0.52f).sp)
    }
}

/**
 * A **medallion** — the rounded parchment tile that frames a quest/reward icon. Pass the stored
 * [icon] (emoji) or let it infer one from [label]; set [reward] to use the treasure icon table.
 */
@Composable
fun Medallion(icon: String?, label: String = "", reward: Boolean = false, modifier: Modifier = Modifier) {
    val glyph = icon ?: inferEmoji(label, reward)
    androidx.compose.foundation.layout.Box(
        modifier = modifier
            .size(44.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(Brush.radialGradient(listOf(SquireParchmentTile, Color(0xFFEAD9AE))))
            .border(1.5.dp, SquireParchmentEdge, RoundedCornerShape(12.dp)),
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
private fun Dot(filled: Boolean) {
    androidx.compose.foundation.layout.Box(
        modifier = Modifier
            .size(11.dp)
            .clip(CircleShape)
            .background(if (filled) SquireGold else MaterialTheme.colorScheme.outlineVariant),
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
