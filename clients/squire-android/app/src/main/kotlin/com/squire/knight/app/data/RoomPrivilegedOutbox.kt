package com.squire.knight.app.data

import com.squire.knight.app.data.db.PrivilegedOutboxDao
import com.squire.knight.app.data.db.PrivilegedOutboxEntity
import com.squire.knight.core.CommandKind
import com.squire.knight.core.KnightCommand
import com.squire.knight.core.PrivilegedOutbox
import com.squire.sdk.model.AdjustReq
import com.squire.sdk.model.MarkDoneReq
import com.squire.sdk.model.RedeemReq
import com.squire.sdk.model.ReviewCashOutReq
import com.squire.sdk.model.ReviewClaimReq
import com.squire.sdk.model.ReviewRedemptionReq
import kotlinx.serialization.json.Json

/**
 * Durable [PrivilegedOutbox] backed by Room. Pending privileged commands survive app/process
 * restart (REQ-K7) and flush on reconnect; idempotency is enforced by the unique `key` index
 * (INSERT OR IGNORE), matching the `:knight-core` contract and the Keep's server-side dedupe.
 */
class RoomPrivilegedOutbox(
    private val dao: PrivilegedOutboxDao,
    private val json: Json,
) : PrivilegedOutbox {

    override fun enqueue(command: KnightCommand) {
        val payloadJson = when (command) {
            is KnightCommand.ReviewClaim ->
                json.encodeToString(ReviewClaimReq.serializer(), command.req)
            is KnightCommand.ReviewRedemption ->
                json.encodeToString(ReviewRedemptionReq.serializer(), command.req)
            is KnightCommand.ReviewCashOut ->
                json.encodeToString(ReviewCashOutReq.serializer(), command.req)
            is KnightCommand.Redeem ->
                json.encodeToString(RedeemReq.serializer(), command.req)
            is KnightCommand.Adjust ->
                json.encodeToString(AdjustReq.serializer(), command.req)
            is KnightCommand.MarkDone ->
                json.encodeToString(MarkDoneReq.serializer(), command.req)
        }
        dao.insert(
            PrivilegedOutboxEntity(key = command.key, kind = command.kind.name, payloadJson = payloadJson),
        )
    }

    override fun pending(): List<KnightCommand> =
        dao.all().map { e ->
            when (CommandKind.valueOf(e.kind)) {
                CommandKind.REVIEW_CLAIM ->
                    KnightCommand.ReviewClaim(json.decodeFromString(ReviewClaimReq.serializer(), e.payloadJson))
                CommandKind.REVIEW_REDEMPTION ->
                    KnightCommand.ReviewRedemption(json.decodeFromString(ReviewRedemptionReq.serializer(), e.payloadJson))
                CommandKind.REVIEW_CASHOUT ->
                    KnightCommand.ReviewCashOut(json.decodeFromString(ReviewCashOutReq.serializer(), e.payloadJson))
                CommandKind.REDEEM ->
                    KnightCommand.Redeem(json.decodeFromString(RedeemReq.serializer(), e.payloadJson))
                CommandKind.ADJUST ->
                    KnightCommand.Adjust(json.decodeFromString(AdjustReq.serializer(), e.payloadJson))
                CommandKind.MARK_DONE ->
                    KnightCommand.MarkDone(json.decodeFromString(MarkDoneReq.serializer(), e.payloadJson))
            }
        }

    override fun markResolved(keys: Set<String>) {
        dao.deleteByKeys(keys.toList())
    }
}
