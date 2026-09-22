package com.squire.core

import com.squire.sdk.model.ClaimState
import com.squire.sdk.model.ClaimStateKind
import com.squire.sdk.model.ClaimStatus
import com.squire.sdk.model.NotifySettings
import com.squire.sdk.model.QuestCard
import com.squire.sdk.model.QuestStatus

/**
 * **Whether to speak, and what to say** — the pure half of notifications (SQUIRE-T-0138, ADR
 * SQUIRE-A-0017). Everything here is a plain function over plain data so it can be tested on the
 * JVM; posting to Android lives in the app's `Notifier`.
 *
 * Three rules shape all of it:
 *  * **Quiet hours.** A squire's phone says nothing outside the household's waking window, judged in
 *    the *household's* timezone, not the device's.
 *  * **Never twice.** Polling sees the same state again and again, so every notification carries a
 *    key and a key is spoken once. A worker that re-announced the same sealed chore every fifteen
 *    minutes would be worse than silence.
 *  * **Nothing on the first sight.** With no previous poll to compare against, everything looks new.
 *    The first run records and stays quiet.
 */
object NotifyPolicy {

    /** The household's waking window when the server hasn't said (older server, or no config yet). */
    const val DEFAULT_WAKE_FROM = 7 * 60
    const val DEFAULT_WAKE_TO = 20 * 60

    /**
     * Is [minuteOfDay] inside the waking window? Both ends come from the household config, so both
     * are trusted only as far as they make sense: an empty or inverted window would silence the
     * whole day, which is never what someone meant, so it falls back to the default.
     *
     * A window that wraps past midnight (`from` after `to`, e.g. 20:00–07:00) is read as spanning
     * midnight rather than as an error — it is a legible thing to ask for.
     */
    fun inWakingHours(
        minuteOfDay: Int,
        wakeFrom: Int = DEFAULT_WAKE_FROM,
        wakeTo: Int = DEFAULT_WAKE_TO,
    ): Boolean {
        val from = wakeFrom.takeIf { it in 0..1439 } ?: DEFAULT_WAKE_FROM
        val to = wakeTo.takeIf { it in 0..1439 } ?: DEFAULT_WAKE_TO
        if (from == to) return true // a zero-width window means "no quiet hours", not "always quiet"
        return if (from < to) minuteOfDay in from until to
        else minuteOfDay >= from || minuteOfDay < to // wraps midnight
    }

    /** The waking window a [NotifySettings] describes, or the defaults when the server sent none. */
    fun wakingWindow(settings: NotifySettings?): Pair<Int, Int> =
        (settings?.wakeFromMinutes ?: DEFAULT_WAKE_FROM) to (settings?.wakeToMinutes ?: DEFAULT_WAKE_TO)

    /** What a grown-up decided about one turn-in — the thing the squire is told. */
    data class Outcome(
        /** De-duplication key; spoken once, ever. */
        val key: String,
        val questTitle: String,
        val sealed: Boolean,
        /** Coins awarded on a seal; `0` when unknown or on a "not yet". */
        val coins: Int = 0,
        /** The Knight's words on a "not yet", when they gave any. */
        val reason: String? = null,
    )

    /**
     * Claims that reached a verdict since [previous] was recorded.
     *
     * [previous] is the set of `"<claimId>:<state>"` markers from the last poll — `null` on a first
     * run, which yields nothing (see "nothing on the first sight"). A claim already terminal in
     * [previous] is not repeated, so a queue the child has already been told about stays quiet.
     */
    fun outcomes(previous: Set<String>?, current: List<ClaimStatus>): List<Outcome> {
        if (previous == null) return emptyList()
        return current.mapNotNull { claim ->
            val marker = marker(claim)
            if (marker in previous) return@mapNotNull null
            when (claim.state.state) {
                ClaimStateKind.Approved -> Outcome(
                    key = "sealed:${claim.claimId}",
                    questTitle = claim.questTitle,
                    sealed = true,
                    coins = claim.state.points ?: 0,
                )
                ClaimStateKind.Rejected -> Outcome(
                    key = "notyet:${claim.claimId}",
                    questTitle = claim.questTitle,
                    sealed = false,
                    reason = claim.state.reason?.trim()?.takeIf { it.isNotEmpty() },
                )
                // Still waiting on a grown-up: nothing to say yet.
                else -> null
            }
        }
    }

    /** The markers to carry into the next poll. */
    fun markers(claims: List<ClaimStatus>): Set<String> = claims.map(::marker).toSet()

    private fun marker(claim: ClaimStatus) = "${claim.claimId}:${claim.state.state.value}"

    /**
     * Does a rise in the coin balance still need announcing on its own?
     *
     * A seal already says "+10 coins", so announcing the same coins twice would be noise. Only a
     * rise the sealed claims do **not** account for — a grown-up granting coins directly, a hazard
     * refund — is worth its own word.
     */
    fun unexplainedCoinGain(previousBalance: Int, currentBalance: Int, sealed: List<Outcome>): Int {
        if (previousBalance < 0 || currentBalance <= previousBalance) return 0
        val explained = sealed.filter { it.sealed }.sumOf { it.coins }
        return (currentBalance - previousBalance - explained).coerceAtLeast(0)
    }

    /**
     * What is still to do today — the reminder's whole reason to exist (SQUIRE-T-0140).
     *
     * A quest counts as left only while it is genuinely actionable: claimed-and-waiting or already
     * sealed are done as far as the child is concerned, and one taken by a sibling is not theirs.
     */
    fun unfinishedToday(quests: List<QuestCard>): List<QuestCard> =
        quests.filter { it.status == QuestStatus.Available }

    /**
     * The chore-time nudge, or `null` to stay silent.
     *
     * Silence is the important half. A reminder that fires when there is nothing left is how a child
     * learns to ignore the app, so an empty list says nothing at all — and because the phone reads
     * its **cached** state, this stays true with the server unreachable.
     */
    fun choreReminder(quests: List<QuestCard>): Pair<String, String>? {
        val left = unfinishedToday(quests)
        return when (left.size) {
            0 -> null
            1 -> "1 chore left today" to left.single().title
            else -> "${left.size} chores left today" to left.joinToString(" · ") { it.title }
        }
    }

    /**
     * Minutes until [minuteOfDay] next comes round, given the current [nowMinuteOfDay] — today if it
     * is still ahead, otherwise tomorrow. Pure arithmetic so the scheduler's one piece of reasoning
     * is testable (SQUIRE-T-0140).
     */
    fun minutesUntilNext(minuteOfDay: Int, nowMinuteOfDay: Int): Int {
        val delta = minuteOfDay - nowMinuteOfDay
        return if (delta > 0) delta else delta + 24 * 60
    }

    /** Keys not yet spoken on this device. */
    fun unspoken(keys: List<String>, alreadySpoken: Set<String>): List<String> =
        keys.filterNot { it in alreadySpoken }
}

/** Convenience: build a [ClaimState] marker string the same way [NotifyPolicy] does. */
internal fun ClaimState.kindValue(): String = state.value
