package com.squire.sdk

import com.squire.sdk.infrastructure.Serializer
import com.squire.sdk.model.ClaimState
import com.squire.sdk.model.ClaimStateKind
import com.squire.sdk.model.DecisionDto
import com.squire.sdk.model.DecisionKind
import com.squire.sdk.model.LockReason
import com.squire.sdk.model.LockReasonKind
import com.squire.sdk.model.RedemptionState
import com.squire.sdk.model.RedemptionStateKind
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Round-trip proof for SQUIRE-T-0033: the generated `ClaimState` / `RedemptionState` / `LockReason`
 * data classes decode the REAL flat tagged JSON the Rust server emits — including the unit variants
 * (`Pending` / `OutOfStock`) that the old discriminator-less `oneOf` collapsed into a broken class.
 *
 * Decoding goes through the SDK's own [Serializer.kotlinxSerializationJson] (the same `Json` the
 * generated API client uses: `ignoreUnknownKeys = true`, the contextual adapters registered).
 */
class SdkDecodeTest {

    private val json = Serializer.kotlinxSerializationJson

    @Test
    fun decodesApprovedClaimState() {
        val claim = json.decodeFromString<ClaimState>("""{"state":"Approved","points":5}""")
        assertEquals(ClaimStateKind.Approved, claim.state)
        assertEquals(5, claim.points)
        assertNull(claim.reason)
    }

    @Test
    fun decodesPendingClaimState() {
        // The unit variant — the case the broken merged class could not represent.
        val claim = json.decodeFromString<ClaimState>("""{"state":"Pending"}""")
        assertEquals(ClaimStateKind.Pending, claim.state)
        assertNull(claim.points)
        assertNull(claim.reason)
    }

    @Test
    fun decodesRejectedClaimStateWithReason() {
        val claim = json.decodeFromString<ClaimState>("""{"state":"Rejected","reason":"too late"}""")
        assertEquals(ClaimStateKind.Rejected, claim.state)
        assertEquals("too late", claim.reason)
        assertNull(claim.points)
    }

    @Test
    fun decodesPendingRedemptionState() {
        val redemption = json.decodeFromString<RedemptionState>("""{"state":"Pending"}""")
        assertEquals(RedemptionStateKind.Pending, redemption.state)
        assertNull(redemption.reason)
    }

    @Test
    fun decodesNeedsAchievementLockReason() {
        val lock = json.decodeFromString<LockReason>(
            """{"kind":"NeedsAchievement","name":"Read 5 books"}""",
        )
        assertEquals(LockReasonKind.NeedsAchievement, lock.kind)
        assertEquals("Read 5 books", lock.name)
    }

    @Test
    fun decodesOutOfStockLockReason() {
        // The other unit variant.
        val lock = json.decodeFromString<LockReason>("""{"kind":"OutOfStock"}""")
        assertEquals(LockReasonKind.OutOfStock, lock.kind)
        assertNull(lock.name)
    }

    // ── SQUIRE-T-0038: DecisionDto round-trips after the flat-struct fix ──────────────────────
    // The old externally-tagged enum (`"approve"` string vs `{"reject":{…}}` object) generated a
    // Kotlin class that could not represent Approve at all; the Knight could never approve.

    @Test
    fun encodesApproveDecisionAsFlatVerdict() {
        // The Knight ENCODES this into the review request body — the case the broken class dropped.
        // (The SDK's Json emits an explicit `"reason":null`, which the server reads as None.)
        val encoded = json.encodeToString(DecisionDto.serializer(), DecisionDto(verdict = DecisionKind.approve))
        assertTrue(encoded.contains(""""verdict":"approve""""), "encoded was: $encoded")
        // And it round-trips back to Approve regardless of the null reason.
        assertEquals(DecisionKind.approve, json.decodeFromString<DecisionDto>(encoded).verdict)
    }

    @Test
    fun decodesApproveDecision() {
        val d = json.decodeFromString<DecisionDto>("""{"verdict":"approve"}""")
        assertEquals(DecisionKind.approve, d.verdict)
        assertNull(d.reason)
    }

    @Test
    fun roundTripsRejectDecisionWithReason() {
        val original = DecisionDto(verdict = DecisionKind.reject, reason = "room still messy")
        val wire = json.encodeToString(DecisionDto.serializer(), original)
        assertTrue(wire.contains(""""verdict":"reject"""") && wire.contains(""""reason":"room still messy""""))
        val back = json.decodeFromString<DecisionDto>(wire)
        assertEquals(DecisionKind.reject, back.verdict)
        assertEquals("room still messy", back.reason)
    }
}
