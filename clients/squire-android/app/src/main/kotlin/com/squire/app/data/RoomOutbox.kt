package com.squire.app.data

import com.squire.app.data.db.OutboxDao
import com.squire.app.data.db.OutboxEntity
import com.squire.core.Outbox
import com.squire.core.OutboxItem
import com.squire.core.SubmissionKind
import com.squire.sdk.model.RequestCashOutReq
import com.squire.sdk.model.RequestRedemptionReq
import com.squire.sdk.model.SubmitClaimReq
import kotlinx.serialization.json.Json

/**
 * Durable [Outbox] backed by Room. Pending offline submissions survive app/process restart
 * (REQ-SY2). Idempotency is enforced by the primary-key id (INSERT OR IGNORE), matching the
 * `:core` contract and the server's idempotency on the phone-minted id.
 */
class RoomOutbox(
    private val dao: OutboxDao,
    private val json: Json,
) : Outbox {

    override fun enqueue(item: OutboxItem) {
        val payloadJson = when (item) {
            is OutboxItem.Claim ->
                json.encodeToString(SubmitClaimReq.serializer(), item.req)
            is OutboxItem.Redemption ->
                json.encodeToString(RequestRedemptionReq.serializer(), item.req)
            is OutboxItem.CashOut ->
                json.encodeToString(RequestCashOutReq.serializer(), item.req)
        }
        dao.insert(OutboxEntity(id = item.id, kind = item.kind.name, payloadJson = payloadJson))
    }

    override fun pending(): List<OutboxItem> =
        dao.all().map { e ->
            when (SubmissionKind.valueOf(e.kind)) {
                SubmissionKind.CLAIM ->
                    OutboxItem.Claim(json.decodeFromString(SubmitClaimReq.serializer(), e.payloadJson))
                SubmissionKind.REDEMPTION ->
                    OutboxItem.Redemption(json.decodeFromString(RequestRedemptionReq.serializer(), e.payloadJson))
                SubmissionKind.CASHOUT ->
                    OutboxItem.CashOut(json.decodeFromString(RequestCashOutReq.serializer(), e.payloadJson))
            }
        }

    override fun markResolved(ids: Set<Long>) {
        dao.deleteByIds(ids.toList())
    }
}
