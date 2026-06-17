package com.squire.knight.core

import com.squire.sdk.model.AdjustReq
import com.squire.sdk.model.MarkDoneReq
import com.squire.sdk.model.RedeemReq
import com.squire.sdk.model.ReviewClaimReq
import com.squire.sdk.model.ReviewRedemptionReq

/** The kind of privileged quick-action an outbox entry represents (SQUIRE-S-0006). */
enum class CommandKind { REVIEW_CLAIM, REVIEW_REDEMPTION, REDEEM, ADJUST, MARK_DONE }

/**
 * A single privileged command the parent issued, wrapping the generated `:sdk` request DTO so the
 * wire shape is the contract's, not hand-rolled.
 *
 * Every command carries a **client-minted idempotency id** ([id]) — `claim_id` / `request_id`
 * (natural keys for reviews and mark-done) or a minted `command_id` (direct redeem / adjust, which
 * lack a natural key; ADR SQUIRE-A-0001). The Keep dedupes on it, so a retried outbox is safe
 * (REQ-K8 / NFR-3).
 *
 * [key] composes [kind] with [id] so two different command kinds that happen to mint the same Long
 * can never collide in the outbox map.
 */
sealed interface KnightCommand {
    val id: Long
    val kind: CommandKind
    val key: String get() = "${kind.name}:$id"

    /** Approve/reject a pending completion claim; the Squire is derived from the claim (REQ-K2). */
    data class ReviewClaim(val req: ReviewClaimReq) : KnightCommand {
        override val id: Long get() = req.claimId
        override val kind: CommandKind get() = CommandKind.REVIEW_CLAIM
    }

    /** Approve/reject a pending redemption request; the Squire is derived from the request (REQ-K4). */
    data class ReviewRedemption(val req: ReviewRedemptionReq) : KnightCommand {
        override val id: Long get() = req.requestId
        override val kind: CommandKind get() = CommandKind.REVIEW_REDEMPTION
    }

    /** Direct redeem on a target Squire's behalf; deduped on the minted `command_id` (REQ-K5). */
    data class Redeem(val req: RedeemReq) : KnightCommand {
        override val id: Long get() = req.commandId
        override val kind: CommandKind get() = CommandKind.REDEEM
    }

    /** Add funds (positive, reason required) to a Squire; deduped on `command_id` (REQ-K6). */
    data class Adjust(val req: AdjustReq) : KnightCommand {
        override val id: Long get() = req.commandId
        override val kind: CommandKind get() = CommandKind.ADJUST
    }

    /** Mark a quest done for a Squire = submit-then-approve; the minted `claim_id` is the key (REQ-K3). */
    data class MarkDone(val req: MarkDoneReq) : KnightCommand {
        override val id: Long get() = req.claimId
        override val kind: CommandKind get() = CommandKind.MARK_DONE
    }
}
