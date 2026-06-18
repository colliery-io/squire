package com.squire.app.screenshots

import app.cash.paparazzi.DeviceConfig
import app.cash.paparazzi.Paparazzi
import com.android.resources.NightMode
import com.squire.app.ui.PlayerHomeScreen
import com.squire.app.ui.theme.SquireTheme
import com.squire.core.PlayerUiState
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.knight.app.ui.KnightHomeScreen
import com.squire.knight.app.ui.LibraryQuest
import com.squire.knight.app.ui.QuestAdminScreen
import com.squire.knight.core.KnightUiState
import com.squire.sdk.model.CompletionDto
import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.ItemOption
import com.squire.sdk.model.PendingClaim
import com.squire.sdk.model.PendingRequest
import com.squire.sdk.model.QuestCard
import com.squire.sdk.model.QuestOption
import com.squire.sdk.model.QuestStatus
import com.squire.sdk.model.QuestSummaryDto
import com.squire.sdk.model.RewardCard
import com.squire.sdk.model.StateView
import com.squire.sdk.model.StreakView
import com.squire.sdk.model.SquireSummary
import org.junit.Rule
import org.junit.Test

/**
 * JVM screenshot tests (SQUIRE-T-0070): render the app's Compose screens to PNGs via Paparazzi — no
 * emulator, no adb, no server. `./gradlew :app:recordPaparazziDebug` writes the goldens under
 * `app/src/test/snapshots/images/`; `:app:verifyPaparazziDebug` fails on a pixel diff. The tightened
 * loop: change a screen → record → review the image.
 */
class ScreenshotTests {

    @get:Rule
    val paparazzi = Paparazzi(
        deviceConfig = DeviceConfig.PIXEL_6.copy(nightMode = NightMode.NOTNIGHT),
    )

    @Test
    fun knightReviewHome() {
        val review = HouseholdReview(
            generatedAt = 0L,
            squires = listOf(
                SquireSummary(balance = 25, displayName = "Gawain", squire = 2),
                SquireSummary(balance = 40, displayName = "Percival", squire = 3),
            ),
            pendingClaims = listOf(
                PendingClaim(claimId = 9L, on = 20624, questTitle = "Tidy your room", squire = 2),
            ),
            pendingRequests = listOf(
                PendingRequest(cost = 15, itemName = "Movie night", requestId = 5L, squire = 2),
            ),
            items = listOf(ItemOption(itemId = 200, name = "Ice cream", cost = 3)),
            quests = listOf(QuestOption(questId = 100, title = "Make your bed")),
            today = 20624,
        )
        paparazzi.snapshot {
            SquireTheme {
                KnightHomeScreen(
                    state = KnightUiState.Ready(review, fromCache = false),
                    onRefresh = {}, onApproveClaim = {}, onRejectClaim = {},
                    onApproveRequest = {}, onRejectRequest = {}, onAddFunds = { _, _, _ -> },
                    onRedeem = { _, _ -> }, onMarkDone = { _, _, _ -> },
                )
            }
        }
    }

    @Test
    fun squirePlayerHome() {
        val view = StateView(
            balance = 12,
            generatedAt = 0L,
            squire = 1L,
            questsToday = listOf(
                QuestCard(on = 1, questId = 1L, reward = 5, status = QuestStatus.Available, title = "Make your bed", icon = "🛏"),
                QuestCard(on = 1, questId = 2L, reward = 10, status = QuestStatus.Pending, title = "Tidy your room", icon = "🧹"),
            ),
            rewards = listOf(
                RewardCard(affordable = true, cost = 3, itemId = 1L, name = "Ice cream", icon = "🍦"),
                RewardCard(affordable = false, cost = 15, itemId = 2L, name = "Movie night", icon = "🎬"),
            ),
            myClaims = emptyList(),
            myRequests = emptyList(),
            streaks = listOf(StreakView(name = "Room Master", current = 3, best = 5, alive = true, nextMilestone = 7)),
        )
        paparazzi.snapshot {
            SquireTheme {
                PlayerHomeScreen(
                    state = PlayerUiState.Ready(view, fromCache = false),
                    onRefresh = {}, onMarkDone = {}, onRedeem = {},
                )
            }
        }
    }

    @Test
    fun knightManageQuests() {
        // A dummy adapter (no network — `initialQuests` is injected so the live fetch is skipped).
        val adapter = KnightApiAdapter(baseUrl = "http://localhost", household = "demo", token = "t")
        val quests = listOf(
            QuestSummaryDto(id = 100, title = "Make your bed", reward = 5, category = "Bedroom",
                cadenceLabel = "Daily", assignmentLabel = "All squires", completion = CompletionDto.EachAssignee,
                repeatableWithinDay = false, autoApprove = true, active = true),
            QuestSummaryDto(id = 101, title = "Take out the trash", reward = 5, category = "Outdoor",
                cadenceLabel = "Mon/Wed/Fri", assignmentLabel = "Gawain", completion = CompletionDto.Race,
                repeatableWithinDay = false, autoApprove = false, active = true),
        )
        val library = listOf(
            LibraryQuest(category = "Pets", title = "Walk the dog", reward = 8, cadence = "daily"),
            LibraryQuest(category = "Kitchen", title = "Clear the table", reward = 5, cadence = "daily"),
        )
        paparazzi.snapshot {
            SquireTheme {
                QuestAdminScreen(
                    adapter = adapter,
                    squires = listOf(2L to "Gawain", 3L to "Percival"),
                    onBack = {},
                    initialQuests = quests,
                    libraryOverride = library,
                )
            }
        }
    }
}
