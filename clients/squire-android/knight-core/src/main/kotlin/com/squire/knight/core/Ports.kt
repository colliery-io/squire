package com.squire.knight.core

import com.squire.sdk.model.HouseholdReview

/**
 * The terminal outcome of submitting one privileged command. The transport adapter (in `:knight-app`)
 * maps okhttp/HTTP status onto this so `:knight-core` stays free of any HTTP dependency.
 *
 * The distinction drives [KnightSyncEngine]'s draining: [Ack] and [AlreadyDone] resolve the item,
 * [Poison] drops it (it can never succeed), and [Offline] stops the drain with the outbox intact.
 */
sealed interface SubmitResult {
    /** 2xx — the Keep committed the command. */
    data object Ack : SubmitResult

    /**
     * Terminal-benign: the action was already applied by someone/something else — a re-review
     * (`AlreadyReviewed` → 409) or a `Race` occurrence already won (`OccurrenceTaken` → 403).
     * Treated as resolved (no double-payout, nothing left to do).
     */
    data object AlreadyDone : SubmitResult

    /** Network failure / Keep unreachable — keep the item, stop draining (REQ-K10 / NFR-1). */
    data object Offline : SubmitResult

    /** A non-retryable client error (e.g. 400 blank reason, 404 not found) — drop so it can't loop. */
    data class Poison(val status: Int, val message: String?) : SubmitResult
}

/**
 * Outbound port that submits one privileged command and reports a [SubmitResult]. Implementations
 * never throw — a thrown transport error is mapped to [SubmitResult.Offline] (or [SubmitResult.Poison]
 * for a definite client error) by the adapter.
 */
interface KnightSubmit {
    suspend fun submit(command: KnightCommand): SubmitResult
}

/**
 * Inbound port fetching the cross-Squire [HouseholdReview] (`GET /household-review`).
 * Implementations throw on network failure; [KnightStore] catches it and falls back to cache.
 */
interface ReviewFetcher {
    suspend fun fetch(): HouseholdReview
}
