package com.squire.app.bg

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.core.app.NotificationCompat
import com.squire.app.MainActivity
import com.squire.core.NotifyPolicy
import com.squire.sdk.model.NotifySettings
import java.time.ZoneId
import java.time.ZonedDateTime

/**
 * The one place a notification is posted (SQUIRE-T-0138). Channels, the quiet-hours gate, the
 * never-say-it-twice ledger and the tap-through target all live here, so a worker only has to decide
 * *what* is worth saying — that decision is pure, and tested, in [NotifyPolicy].
 *
 * Posting is always best-effort: without the POST_NOTIFICATIONS grant (API 33+) `notify` throws and
 * we swallow it, exactly as the original workers did. A missing grant must never fail a background
 * job.
 */
object Notifier {

    /** The categories a household can silence independently in Android's own settings. */
    enum class Channel(val id: String, val label: String) {
        /** A grown-up's verdict, and coins landing. */
        Seals("squire_seals", "Seals & coins"),

        /** Timed nudges: chore time, a streak about to lapse. */
        Reminders("squire_reminders", "Reminders"),

        /** A new app build is ready to install. */
        Updates("squire_updates", "App updates"),
    }

    /** Which screen a tap should open. Read by `MainActivity`. */
    const val EXTRA_TAB = "com.squire.app.TAB"

    /**
     * Post [title] / [text] unless this device has already said it, or the household is asleep.
     *
     * @param key the de-duplication key — spoken once, ever (see [NotifyPolicy]).
     * @param settings the household's frame; `null` falls back to the built-in waking window.
     * @param respectQuietHours false for a Knight: an adult manages their own phone (SQUIRE-I-0007).
     * @return true when something was actually posted.
     */
    fun post(
        ctx: Context,
        channel: Channel,
        key: String,
        title: String,
        text: String,
        tab: String? = null,
        settings: NotifySettings? = null,
        respectQuietHours: Boolean = true,
        notificationId: Int = key.hashCode(),
    ): Boolean {
        if (hasSpoken(ctx, key)) return false
        if (respectQuietHours && !isAwake(settings)) return false

        val mgr = ctx.getSystemService(NotificationManager::class.java) ?: return false
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            mgr.createNotificationChannel(
                NotificationChannel(channel.id, channel.label, NotificationManager.IMPORTANCE_DEFAULT),
            )
        }
        val intent = Intent(ctx, MainActivity::class.java)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
            .apply { if (tab != null) putExtra(EXTRA_TAB, tab) }
        val open = PendingIntent.getActivity(
            ctx,
            // Distinct per target, or Android reuses the first intent for every later notification.
            tab.hashCode(),
            intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val notification = NotificationCompat.Builder(ctx, channel.id)
            .setSmallIcon(android.R.drawable.btn_star_big_on)
            .setContentTitle(title)
            .setContentText(text)
            .setStyle(NotificationCompat.BigTextStyle().bigText(text))
            .setAutoCancel(true)
            .setContentIntent(open)
            .build()
        // POST_NOTIFICATIONS (API 33+): if the grant is missing the post is a silent no-op.
        val posted = runCatching { mgr.notify(notificationId, notification) }.isSuccess
        if (posted) remember(ctx, key)
        return posted
    }

    /**
     * Is the household awake right now? Judged in the **household's** timezone: a child on holiday in
     * another zone should still be quiet at their family's bedtime, and the server is the only thing
     * that knows which zone that is.
     */
    fun isAwake(settings: NotifySettings?, now: ZonedDateTime? = null): Boolean {
        val zone = settings?.timezone
            ?.let { tz -> runCatching { ZoneId.of(tz) }.getOrNull() }
            ?: ZoneId.systemDefault()
        val at = now ?: ZonedDateTime.now(zone)
        val (from, to) = NotifyPolicy.wakingWindow(settings)
        return NotifyPolicy.inWakingHours(at.hour * 60 + at.minute, from, to)
    }

    // ── the ledger ─────────────────────────────────────────────────────────────────────────────

    /**
     * Keys already spoken on this device. Bounded: only the most recent [LEDGER_CAP] are kept, which
     * is far more than a household generates in the window where a repeat would be noticed, and
     * stops a long-lived install growing this file forever.
     */
    private fun hasSpoken(ctx: Context, key: String) = key in ledger(ctx)

    /** Everything this device has already said — for callers that decide in bulk (SQUIRE-T-0139). */
    fun spoken(ctx: Context): Set<String> = ledger(ctx).toSet()

    fun remember(ctx: Context, key: String) {
        val kept = (ledger(ctx) + key).takeLast(LEDGER_CAP)
        prefs(ctx).edit().putString(KEY_LEDGER, kept.joinToString("\n")).apply()
    }

    private fun ledger(ctx: Context): List<String> =
        prefs(ctx).getString(KEY_LEDGER, "").orEmpty().split("\n").filter { it.isNotEmpty() }

    /** Forget everything this device has said — on unpair, so a re-paired phone starts clean. */
    fun clear(ctx: Context) {
        prefs(ctx).edit().remove(KEY_LEDGER).apply()
    }

    private fun prefs(ctx: Context) = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    private const val PREFS = "squire_notify"
    private const val KEY_LEDGER = "spoken"
    private const val LEDGER_CAP = 200
}
