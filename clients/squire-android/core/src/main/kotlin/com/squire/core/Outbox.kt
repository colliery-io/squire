package com.squire.core

import com.squire.sdk.model.RequestRedemptionReq
import com.squire.sdk.model.SubmitClaimReq

/** The kind of submission an outbox item represents. */
enum class SubmissionKind { CLAIM, REDEMPTION }

/**
 * A single pending offline submission.
 *
 * [id] is the phone-minted idempotency key (`claimId` for claims, `requestId` for
 * redemptions). The server is idempotent on this id, so re-posting is always safe.
 */
sealed interface OutboxItem {
    val id: Long
    val kind: SubmissionKind

    data class Claim(val req: SubmitClaimReq) : OutboxItem {
        override val id: Long get() = req.claimId
        override val kind: SubmissionKind get() = SubmissionKind.CLAIM
    }

    data class Redemption(val req: RequestRedemptionReq) : OutboxItem {
        override val id: Long get() = req.requestId
        override val kind: SubmissionKind get() = SubmissionKind.REDEMPTION
    }
}

/**
 * An idempotent, order-preserving queue of pending submissions, keyed by the
 * phone-minted id. Re-enqueueing a known id is a no-op (REQ-SY: idempotent outbox).
 */
interface Outbox {
    /** Add an item. If its id is already present, this is a no-op. */
    fun enqueue(item: OutboxItem)

    /** Pending items, in insertion order. */
    fun pending(): List<OutboxItem>

    /** Remove items whose ids the server has now acknowledged. */
    fun markResolved(ids: Set<Long>)
}

/** In-memory [Outbox] backed by an insertion-ordered map keyed by item id. */
class InMemoryOutbox : Outbox {
    private val items = LinkedHashMap<Long, OutboxItem>()

    override fun enqueue(item: OutboxItem) {
        items.putIfAbsent(item.id, item)
    }

    override fun pending(): List<OutboxItem> = items.values.toList()

    override fun markResolved(ids: Set<Long>) {
        ids.forEach { items.remove(it) }
    }
}
