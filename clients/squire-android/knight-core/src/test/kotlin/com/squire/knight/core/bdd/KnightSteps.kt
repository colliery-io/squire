package com.squire.knight.core.bdd

import com.squire.knight.core.InMemoryPrivilegedOutbox
import com.squire.knight.core.InMemoryReviewCache
import com.squire.knight.core.KnightCommand
import com.squire.knight.core.KnightStore
import com.squire.knight.core.KnightSubmit
import com.squire.knight.core.KnightSyncEngine
import com.squire.knight.core.ReviewFetcher
import com.squire.knight.core.SubmitResult
import com.squire.sdk.model.Currency
import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.SquireSummary
import io.cucumber.java.en.Then
import io.cucumber.java.en.When
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.Json
import java.util.concurrent.atomic.AtomicLong
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

/**
 * Step definitions for the Knight quick-action features (SQUIRE-T-0115). One instance per scenario,
 * so `submitted` accumulates the commands the store enqueued. Uses the in-memory doubles (main
 * source set) + a capturing [KnightSubmit] — the same shape as KnightStoreTest, as Gherkin.
 */
class KnightSteps {
    private val submitted = mutableListOf<KnightCommand>()

    private val submit = object : KnightSubmit {
        override suspend fun submit(command: KnightCommand): SubmitResult {
            submitted += command
            return SubmitResult.Ack
        }
    }

    private val fetcher = object : ReviewFetcher {
        override suspend fun fetch(): HouseholdReview = HouseholdReview(
            generatedAt = 1L,
            pendingClaims = emptyList(),
            pendingRequests = emptyList(),
            squires = listOf(SquireSummary(balance = 0, displayName = "Gawain", squire = 2)),
            items = emptyList(),
            quests = emptyList(),
            today = 20624,
        )
    }

    private val outbox = InMemoryPrivilegedOutbox()
    private val store = KnightStore(
        fetcher,
        InMemoryReviewCache(),
        outbox,
        KnightSyncEngine(outbox, submit),
        Json { ignoreUnknownKeys = true },
        AtomicLong(1000)::getAndIncrement,
    )

    private val ids = mapOf("Gawain" to 2L)

    private fun lastAdjust(): KnightCommand.Adjust =
        submitted.filterIsInstance<KnightCommand.Adjust>().lastOrNull()
            ?: error("no Adjust command was submitted")

    @When("the knight pays {word} {int} dollars with reason {string}")
    fun pays(name: String, amount: Int, reason: String) = runBlocking {
        store.pay(ids.getValue(name), amount.toLong(), reason)
    }

    @When("the knight grants {word} {int} coins with reason {string}")
    fun grants(name: String, amount: Int, reason: String) = runBlocking {
        store.adjust(ids.getValue(name), amount.toLong(), reason)
    }

    @Then("a Cash adjustment of {int} is submitted")
    fun cashSubmitted(amount: Int) {
        val cmd = lastAdjust()
        assertEquals(Currency.Cash, cmd.req.currency, "currency should be Cash")
        assertEquals(amount.toLong(), cmd.req.amount, "amount")
    }

    @Then("a Coins adjustment of {int} is submitted")
    fun coinsSubmitted(amount: Int) {
        val cmd = lastAdjust()
        // Coins is the default — the store leaves currency null (the server resolves null → Coins).
        assertTrue(cmd.req.currency == null || cmd.req.currency == Currency.Coins, "currency should be Coins/null")
        assertEquals(amount.toLong(), cmd.req.amount, "amount")
    }

    @Then("paying {word} {int} dollars with reason {string} is rejected")
    fun payRejected(name: String, amount: Int, reason: String) {
        assertFailsWith<IllegalArgumentException> {
            runBlocking { store.pay(ids.getValue(name), amount.toLong(), reason) }
        }
        assertTrue(outbox.pending().isEmpty(), "a rejected pay must not enqueue anything")
    }
}
