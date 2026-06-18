package com.squire.app.ui.components

import androidx.compose.foundation.background
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.squire.app.ui.theme.SquireGold

/** A rounded gold chip for points/gold — `★ N`. [large] for the hero balance. */
@Composable
fun GoldPill(amount: Int, large: Boolean = false) {
    Surface(
        color = MaterialTheme.colorScheme.secondaryContainer,
        shape = RoundedCornerShape(50),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(horizontal = if (large) 16.dp else 12.dp, vertical = if (large) 8.dp else 5.dp),
        ) {
            Text("★", color = SquireGold, fontSize = if (large) 22.sp else 15.sp, fontWeight = FontWeight.Bold)
            Text(
                "  $amount",
                color = MaterialTheme.colorScheme.onSecondaryContainer,
                fontWeight = FontWeight.Bold,
                fontSize = if (large) 22.sp else 15.sp,
            )
        }
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
            fontWeight = FontWeight.Medium,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
        )
    }
}

/** A section heading in the display serif, royal-colored, with a hairline rule under it. */
@Composable
fun SectionTitle(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.titleMedium,
        color = MaterialTheme.colorScheme.primary,
    )
}

/** A streak progress as filled/empty dots, e.g. ●●●○○○○. */
@Composable
fun ProgressDots(current: Int, target: Int) {
    Row(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
        val total = target.coerceIn(1, 10)
        repeat(total) { i ->
            Box(filled = i < current)
        }
    }
}

@Composable
private fun Box(filled: Boolean) {
    androidx.compose.foundation.layout.Box(
        modifier = Modifier
            .size(10.dp)
            .clip(CircleShape)
            .background(
                if (filled) SquireGold else MaterialTheme.colorScheme.outlineVariant,
            ),
    )
}

/** A soft full-width banner (offline / update / info). */
@Composable
fun Banner(text: String, container: Color, content: Color, trailing: @Composable (() -> Unit)? = null) {
    Surface(color = container, shape = RoundedCornerShape(14.dp)) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
            modifier = Modifier
                .padding(horizontal = 16.dp, vertical = 10.dp),
        ) {
            Text(text, color = content, fontWeight = FontWeight.Medium, modifier = Modifier.weight(1f))
            trailing?.invoke()
        }
    }
}
