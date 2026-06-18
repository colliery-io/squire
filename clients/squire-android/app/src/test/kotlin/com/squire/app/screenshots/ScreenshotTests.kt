package com.squire.app.screenshots

import app.cash.paparazzi.DeviceConfig
import app.cash.paparazzi.Paparazzi
import com.android.resources.NightMode
import com.squire.app.ui.PlayerHomeScreen
import com.squire.app.ui.theme.SquireTheme
import com.squire.core.PlayerUiState
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.knight.app.ui.AchievementAdminScreen
import com.squire.knight.app.ui.KnightHomeScreen
import com.squire.knight.app.ui.LibraryAch
import com.squire.knight.app.ui.LibraryQuest
import com.squire.knight.app.ui.LibraryReward
import com.squire.knight.app.ui.MemberAdminScreen
import com.squire.knight.app.ui.QuestAdminScreen
import com.squire.knight.app.ui.RejectReasonDialog
import com.squire.knight.app.ui.RewardAdminScreen
import com.squire.knight.core.KnightUiState
import com.squire.sdk.model.AchievementSummaryDto
import com.squire.sdk.model.ClaimState
import com.squire.sdk.model.ClaimStateKind
import com.squire.sdk.model.ClaimStatus
import com.squire.sdk.model.CompletionDto
import com.squire.sdk.model.ItemSummaryDto
import com.squire.sdk.model.MemberSummaryDto
import com.squire.sdk.model.Role
import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.ItemOption
import com.squire.sdk.model.PendingClaim
import com.squire.sdk.model.PendingRequest
import com.squire.sdk.model.QuestCard
import com.squire.sdk.model.QuestOption
import com.squire.sdk.model.QuestStatus
import com.squire.sdk.model.QuestSummaryDto
import com.squire.sdk.model.RedemptionState
import com.squire.sdk.model.RedemptionStateKind
import com.squire.sdk.model.RedemptionStatus
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
                    onRefresh = {}, onApproveClaim = {}, onRejectClaim = { _, _ -> },
                    onApproveRequest = {}, onRejectRequest = { _, _ -> }, onAddFunds = { _, _, _ -> },
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

    @Test
    fun knightManageAchievements() {
        // A dummy adapter (no network — `initialAchievements` is injected so the live fetch is skipped).
        val adapter = KnightApiAdapter(baseUrl = "http://localhost", household = "demo", token = "t")
        val achievements = listOf(
            AchievementSummaryDto(id = 1, name = "Century Club", summary = "100 points", bonus = 25, active = true),
            AchievementSummaryDto(id = 2, name = "Tidy Streak", summary = "7-day streak · Bedroom", bonus = 25, active = true),
            AchievementSummaryDto(id = 3, name = "Kitchen Helper", summary = "20 completions · Kitchen", bonus = 20, active = false),
        )
        val library = listOf(
            LibraryAch(name = "Chore Champion", criterion = "total", scope = "any", count = 50, bonus = 50),
            LibraryAch(name = "On a Roll", criterion = "streak", scope = "any", length = 7, basis = "CalendarDays", bonus = 30),
        )
        paparazzi.snapshot {
            SquireTheme {
                AchievementAdminScreen(
                    adapter = adapter,
                    onBack = {},
                    initialAchievements = achievements,
                    libraryOverride = library,
                )
            }
        }
    }

    @Test
    fun knightManageRewards() {
        // A dummy adapter (no network — `initialItems` is injected so the live fetch is skipped).
        val adapter = KnightApiAdapter(baseUrl = "http://localhost", household = "demo", token = "t")
        val items = listOf(
            ItemSummaryDto(id = 1, name = "Extra screen time", cost = 10, summary = "Repeatable", active = true),
            ItemSummaryDto(id = 2, name = "Movie night pick", cost = 25, summary = "Repeatable", active = true),
            ItemSummaryDto(id = 3, name = "Day-trip pick", cost = 100, summary = "Once · needs: Century Club", active = false),
        )
        val library = listOf(
            LibraryReward(name = "Dessert of choice", cost = 8, availability = "Repeatable"),
            LibraryReward(name = "Friend sleepover", cost = 60, availability = "Once"),
        )
        val gates = listOf(1L to "Century Club", 2L to "Tidy Streak")
        paparazzi.snapshot {
            SquireTheme {
                RewardAdminScreen(
                    adapter = adapter,
                    onBack = {},
                    initialItems = items,
                    libraryOverride = library,
                    gateOverride = gates,
                )
            }
        }
    }

    @Test
    fun knightRejectReasonDialog() {
        // The parent can attach a reason when rejecting from the phone (SQUIRE-T-0078) — the dialog
        // the child's rejection note comes from.
        paparazzi.snapshot {
            SquireTheme {
                RejectReasonDialog(
                    title = "Reject quest",
                    subject = "Walk the dog · Gawain",
                    onDismiss = {},
                    onConfirm = {},
                )
            }
        }
    }

    @Test
    fun squirePlayerHomeHistory() {
        // The "Recent" section must surface the outcome of each claim — especially WHY one was
        // rejected (SQUIRE-T-0076). Seeds all three claim states incl. a rejection reason.
        val view = StateView(
            balance = 5,
            generatedAt = 0L,
            squire = 1L,
            questsToday = listOf(
                QuestCard(on = 1, questId = 1L, reward = 5, status = QuestStatus.Available, title = "Make your bed", icon = "🛏"),
            ),
            rewards = emptyList(),
            myClaims = listOf(
                ClaimStatus(claimId = 10L, on = 1, questTitle = "Tidy your room",
                    state = ClaimState(state = ClaimStateKind.Approved, points = 10)),
                ClaimStatus(claimId = 11L, on = 1, questTitle = "Walk the dog",
                    state = ClaimState(state = ClaimStateKind.Rejected, reason = "Bowl wasn't refilled")),
                ClaimStatus(claimId = 12L, on = 1, questTitle = "Take out the trash",
                    state = ClaimState(state = ClaimStateKind.Pending)),
            ),
            myRequests = listOf(
                // A rejected reward request must also show the child WHY (SQUIRE-T-0077).
                RedemptionStatus(cost = 25, itemName = "Movie night", requestId = 20L,
                    state = RedemptionState(state = RedemptionStateKind.Rejected, reason = "After homework")),
            ),
            streaks = emptyList(),
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
    fun knightManageMembers() {
        // A dummy adapter (no network — `initialMembers` is injected so the live fetch is skipped).
        val adapter = KnightApiAdapter(baseUrl = "http://localhost", household = "demo", token = "t")
        val members = listOf(
            MemberSummaryDto(active = true, displayName = "Arthur", role = Role.Knight, user = 1),
            MemberSummaryDto(active = true, displayName = "Gawain", role = Role.Squire, user = 2),
            MemberSummaryDto(active = false, displayName = "Percival", role = Role.Squire, user = 3),
        )
        paparazzi.snapshot {
            SquireTheme {
                MemberAdminScreen(
                    adapter = adapter,
                    selfUser = 1,
                    host = "10.0.0.227",
                    port = 8088,
                    household = "demo",
                    onBack = {},
                    initialMembers = members,
                )
            }
        }
    }
}
