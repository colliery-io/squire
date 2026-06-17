package com.squire.core

import com.squire.sdk.model.ClaimState
import com.squire.sdk.model.ClaimStateKind
import com.squire.sdk.model.ClaimStatus
import com.squire.sdk.model.QuestCard
import com.squire.sdk.model.QuestStatus
import com.squire.sdk.model.RedemptionState
import com.squire.sdk.model.RedemptionStateKind
import com.squire.sdk.model.RedemptionStatus
import com.squire.sdk.model.RequestRedemptionReq
import com.squire.sdk.model.RewardCard
import com.squire.sdk.model.StateView
import com.squire.sdk.model.StreakView
import com.squire.sdk.model.SubmitClaimReq
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import java.io.IOException
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

private val json = Json { ignoreUnknownKeys = true }

/** A small but representative StateView built from the generated constructors. */
private fun sampleState(balance: Int = 120): StateView = StateView(
    balance = balance,
    generatedAt = 1_700_000_000L,
    squire = 7L,
    questsToday = listOf(
        QuestCard(
            on = 1,
            questId = 100L,
            reward = 10,
            status = QuestStatus.Available,
            title = "Make your bed",
        ),
        QuestCard(
            on = 1,
            questId = 101L,
            reward = 15,
            status = QuestStatus.TakenByOther,
            title = "Walk the dog",
        ),
    ),
    rewards = listOf(
        RewardCard(affordable = true, cost = 50, itemId = 200L, name = "Ice cream"),
        RewardCard(affordable = false, cost = 500, itemId = 201L, name = "New game"),
    ),
    myClaims = listOf(
        ClaimStatus(
            claimId = 900L,
            on = 1,
            questTitle = "Make your bed",
            state = ClaimState(state = ClaimStateKind.Pending),
        ),
    ),
    myRequests = listOf(
        RedemptionStatus(
            cost = 50,
            itemName = "Ice cream",
            requestId = 800L,
            state = RedemptionState(state = RedemptionStateKind.Pending),
        ),
    ),
    streaks = listOf(
        StreakView(alive = true, best = 10, current = 3, name = "Daily chores", nextMilestone = 5),
    ),
)

private class FakeFetcher(
    private val view: StateView? = null,
    private val fail: Throwable? = null,
) : StateFetcher {
    override suspend fun fetchState(): StateView {
        fail?.let { throw it }
        return view!!
    }
}

/** Records submission posts; succeeds. */
private class RecordingSubmissionApi : SubmissionApi {
    val claimIds = mutableListOf<Long>()
    val requestIds = mutableListOf<Long>()

    override suspend fun submitClaim(req: SubmitClaimReq) {
        claimIds += req.claimId
    }

    override suspend fun requestRedemption(req: RequestRedemptionReq) {
        requestIds += req.requestId
    }
}

private class FakeResolvedStatePort(var resolved: ResolvedIds = ResolvedIds(emptySet(), emptySet())) :
    StatePort {
    override suspend fun resolvedIds(): ResolvedIds = resolved
}

private fun store(
    fetcher: StateFetcher,
    cache: StateCache = InMemoryStateCache(),
    outbox: Outbox = InMemoryOutbox(),
    api: SubmissionApi = RecordingSubmissionApi(),
    statePort: StatePort = FakeResolvedStatePort(),
    ids: () -> Long = { 42L },
): PlayerStore = PlayerStore(
    fetcher = fetcher,
    cache = cache,
    outbox = outbox,
    sync = SyncEngine(outbox, api, statePort),
    json = json,
    ids = ids,
)

class PlayerStoreTest {

    @Test
    fun `refresh on success emits Ready not-from-cache and writes the cache`() = runTest {
        val view = sampleState()
        val cache = InMemoryStateCache()
        val s = store(FakeFetcher(view = view), cache = cache)

        s.refresh()

        val ready = assertIs<PlayerUiState.Ready>(s.state.value)
        assertEquals(view, ready.view)
        assertEquals(false, ready.fromCache)
        // Cache was populated and round-trips back to the same view.
        val raw = cache.load()
        assertTrue(raw != null)
        assertEquals(view, json.decodeFromString(StateView.serializer(), raw))
    }

    @Test
    fun `refresh offline with a seeded cache emits Ready from-cache`() = runTest {
        val cachedView = sampleState(balance = 999)
        val cache = InMemoryStateCache().apply {
            save(json.encodeToString(StateView.serializer(), cachedView))
        }
        val s = store(FakeFetcher(fail = IOException("computer unreachable")), cache = cache)

        s.refresh()

        val ready = assertIs<PlayerUiState.Ready>(s.state.value)
        assertEquals(cachedView, ready.view)
        assertEquals(true, ready.fromCache)
    }

    @Test
    fun `refresh offline with an empty cache emits Error`() = runTest {
        val s = store(FakeFetcher(fail = IOException("computer unreachable")))

        s.refresh()

        assertIs<PlayerUiState.Error>(s.state.value)
    }

    @Test
    fun `submitClaim enqueues a claim and a sync pass flushes plus reconciles it`() = runTest {
        val outbox = InMemoryOutbox()
        val api = RecordingSubmissionApi()
        // The refreshed state reports the minted claim id 42 as resolved.
        val statePort = FakeResolvedStatePort(ResolvedIds(claims = setOf(42L), requests = emptySet()))
        val s = store(
            FakeFetcher(view = sampleState()),
            outbox = outbox,
            api = api,
            statePort = statePort,
            ids = { 42L },
        )

        s.submitClaim(questId = 100L, on = 1)

        // The claim was posted with the minted id, then reconciled out of the outbox.
        assertEquals(listOf(42L), api.claimIds)
        assertTrue(outbox.pending().isEmpty())
        assertIs<PlayerUiState.Ready>(s.state.value)
    }

    @Test
    fun `requestRedemption enqueues a redemption and stays pending when unresolved`() = runTest {
        val outbox = InMemoryOutbox()
        val api = RecordingSubmissionApi()
        // Server has not yet acknowledged the request id, so it stays pending.
        val statePort = FakeResolvedStatePort(ResolvedIds(claims = emptySet(), requests = emptySet()))
        val s = store(
            FakeFetcher(view = sampleState()),
            outbox = outbox,
            api = api,
            statePort = statePort,
            ids = { 77L },
        )

        s.requestRedemption(itemId = 200L)

        assertEquals(listOf(77L), api.requestIds)
        assertEquals(listOf(77L), outbox.pending().map { it.id })
        assertIs<PlayerUiState.Ready>(s.state.value)
    }
}
