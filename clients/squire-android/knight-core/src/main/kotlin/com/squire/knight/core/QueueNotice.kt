package com.squire.knight.core

import com.squire.sdk.model.HouseholdReview

/**
 * **What to tell a Knight about their queue** (SQUIRE-T-0139) — the pure half, so it can be tested
 * on the JVM. Posting lives in the app's `Notifier`.
 *
 * The parent is the bottleneck in the whole household: nothing is sealed until they look, and until
 * now nothing told them to. The hard part is not noticing that the queue is non-empty — it is *not*
 * saying so every fifteen minutes while it sits there. So this reports only items the device has
 * never announced, and goes quiet again the moment there are none.
 */
object QueueNotice {

    /** One thing waiting, named for the notification. */
    data class Item(val key: String, val squire: String, val what: String)

    data class Notice(val title: String, val text: String, val keys: List<String>)

    /**
     * Everything currently waiting — claims, reward requests and cash-outs, matching what the Review
     * screen counts (SQUIRE-T-0134), with squires named rather than numbered.
     */
    fun waiting(review: HouseholdReview): List<Item> {
        val names = review.squires.associate { it.squire to it.displayName }
        fun who(id: Long) = names[id] ?: "A squire"
        return buildList {
            review.pendingClaims.forEach {
                add(Item("waiting:claim:${it.claimId}", who(it.squire), it.questTitle))
            }
            review.pendingRequests.forEach {
                add(Item("waiting:request:${it.requestId}", who(it.squire), it.itemName))
            }
            review.pendingCashouts.orEmpty().forEach {
                add(Item("waiting:cashout:${it.requestId}", who(it.squire), "a $${it.amount} cash-out"))
            }
        }
    }

    /**
     * What to say, given everything waiting and what this device has already said — or `null` when
     * there is nothing new.
     *
     * The count in the text is the **whole** queue, not just the new part: a parent wants to know
     * what they are walking into, and "3 waiting for your seal" is more useful than "1 new".
     */
    fun notice(review: HouseholdReview, alreadySpoken: Set<String>): Notice? {
        val all = waiting(review)
        val fresh = all.filterNot { it.key in alreadySpoken }
        if (fresh.isEmpty()) return null

        val keys = fresh.map { it.key }
        return if (all.size == 1) {
            val only = all.single()
            Notice("${only.squire} turned in ${only.what}", "Tap to seal it.", keys)
        } else if (fresh.size == 1) {
            val one = fresh.single()
            Notice("${one.squire} turned in ${one.what}", "${all.size} waiting for your seal.", keys)
        } else {
            Notice("${all.size} waiting for your seal", fresh.joinToString(" · ") { "${it.squire}: ${it.what}" }, keys)
        }
    }
}
