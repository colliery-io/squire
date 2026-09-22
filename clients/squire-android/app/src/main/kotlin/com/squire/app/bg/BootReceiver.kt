package com.squire.app.bg

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import com.squire.pairing.SessionStore

/**
 * Re-arm the background work a reboot clears (SQUIRE-T-0140).
 *
 * WorkManager restores its own periodic work, but the chore reminder is a *one-shot* re-armed after
 * each run, so a phone rebooted between two chore times would simply never nudge again. The periodic
 * watchers re-arm it on their next poll, but that could be fifteen minutes of a silent evening; this
 * closes the gap immediately. Also covers a timezone change, where "16:30" now means a different
 * instant.
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action ?: return
        if (action != Intent.ACTION_BOOT_COMPLETED &&
            action != Intent.ACTION_TIMEZONE_CHANGED &&
            action != Intent.ACTION_MY_PACKAGE_REPLACED
        ) {
            return
        }
        val session = SessionStore(context).load() ?: return
        // The watchers re-read the household's settings on their next run and re-arm the reminders
        // from them; all we need here is for the watchers themselves to be running.
        val knight = session.role == "Knight"
        if (knight) KnightNotifyWorker.schedule(context) else SquireNotifyWorker.schedule(context)
        // A reboot ends the one-shot chain outright (SQUIRE-T-0144), so start it again here rather
        // than waiting up to fifteen minutes for the periodic backstop to notice.
        FastPoll.arm(context, knight = knight)
        UpdateCheckWorker.schedule(context)
    }
}
