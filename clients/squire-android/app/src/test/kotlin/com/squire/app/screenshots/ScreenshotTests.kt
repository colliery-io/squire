package com.squire.app.screenshots

import androidx.compose.foundation.background
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.TextButton
import androidx.compose.material3.Snackbar
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import app.cash.paparazzi.DeviceConfig
import app.cash.paparazzi.Paparazzi
import com.android.resources.NightMode
import com.squire.app.ui.DetailCard
import com.squire.app.ui.PlayerHomeScreen
import com.squire.app.ui.SquireTab
import com.squire.app.ui.badgeDetail
import com.squire.app.ui.goalDetail
import com.squire.app.ui.questDetail
import com.squire.app.ui.rewardDetail
import com.squire.app.ui.streakDetail
import com.squire.app.ui.theme.SquireTheme
import com.squire.core.PlayerUiState
import com.squire.knight.app.data.KnightApiAdapter
import com.squire.knight.app.ui.AddFundsFields
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
import com.squire.sdk.model.ActivityEntry
import com.squire.sdk.model.ActivityKind
import com.squire.sdk.model.AdjustmentView
import com.squire.sdk.model.BadgeView
import com.squire.sdk.model.HistoryEntryDto
import com.squire.sdk.model.GoalView
import com.squire.sdk.model.ClaimState
import com.squire.sdk.model.ClaimStateKind
import com.squire.sdk.model.ClaimStatus
import com.squire.sdk.model.CompletionDto
import com.squire.sdk.model.Currency
import com.squire.sdk.model.CurrencyBalance
import com.squire.sdk.model.AvailabilityKind
import com.squire.sdk.model.ItemSummaryDto
import com.squire.sdk.model.MemberSummaryDto
import com.squire.sdk.model.PendingCashOut
import com.squire.sdk.model.Role
import com.squire.sdk.model.HouseholdReview
import com.squire.sdk.model.ItemOption
import com.squire.sdk.model.LockReason
import com.squire.sdk.model.LockReasonKind
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
                SquireSummary(balance = 25, displayName = "Gawain", squire = 2, cashBalance = 5),
                SquireSummary(balance = 40, displayName = "Percival", squire = 3),
            ),
            pendingClaims = listOf(
                PendingClaim(claimId = 9L, on = 20624, questTitle = "Tidy your room", squire = 2, reward = 10),
            ),
            pendingRequests = listOf(
                PendingRequest(cost = 15, itemName = "Movie night", requestId = 5L, squire = 2),
            ),
            pendingCashouts = listOf(
                PendingCashOut(amount = 5, requestId = 7L, squire = 2),
            ),
            items = listOf(ItemOption(itemId = 200, name = "Ice cream", cost = 3)),
            quests = listOf(QuestOption(questId = 100, title = "Make your bed")),
            today = 20624,
        )
        paparazzi.snapshot {
            SquireTheme {
                KnightHomeScreen(
                    state = KnightUiState.Ready(review, fromCache = false),
                    name = "David Storey",
                    onRefresh = {}, onApproveClaim = {}, onRejectClaim = { _, _ -> },
                    onApproveRequest = {}, onRejectRequest = { _, _ -> }, onAddFunds = { _, _, _, _ -> },
                    onPay = { _, _, _ -> },
                    onRedeem = { _, _ -> }, onMarkDone = { _, _, _ -> },
                )
            }
        }
    }

    @Test
    fun knightManage() {
        val review = HouseholdReview(
            generatedAt = 0L, squires = emptyList(), pendingClaims = emptyList(),
            pendingRequests = emptyList(), items = emptyList(), quests = emptyList(), today = 20624,
        )
        paparazzi.snapshot {
            SquireTheme {
                KnightHomeScreen(
                    state = KnightUiState.Ready(review, fromCache = false),
                    name = "David Storey",
                    initialTab = com.squire.knight.app.ui.KnightTab.Manage,
                    onRefresh = {}, onApproveClaim = {}, onRejectClaim = { _, _ -> },
                    onApproveRequest = {}, onRejectRequest = { _, _ -> }, onAddFunds = { _, _, _, _ -> },
                    onPay = { _, _, _ -> },
                    onRedeem = { _, _ -> }, onMarkDone = { _, _, _ -> },
                )
            }
        }
    }

    @Test
    fun knightHistory() {
        val review = HouseholdReview(
            generatedAt = 0L,
            squires = listOf(SquireSummary(balance = 25, displayName = "Gawain", squire = 2)),
            pendingClaims = emptyList(),
            pendingRequests = emptyList(), items = emptyList(), quests = emptyList(), today = 20624,
        )
        val at = 1_700_000_000_000L
        // The server now resolves names/titles (SQUIRE-T-0121): the Squire, the acting Knight, and
        // the quest/reward subject. Rows read "Gawain earned 5 coins for “Make your bed” · approved by Dad".
        val history = listOf(
            HistoryEntryDto(at = at, kind = "CashedOut", squire = 2, squireName = "Gawain", amount = 5, currency = Currency.Cash, actorName = "Dad"),
            HistoryEntryDto(at = at - 60_000, kind = "CashOutRequested", squire = 2, squireName = "Gawain", amount = 5, currency = Currency.Cash),
            HistoryEntryDto(at = at - 120_000, kind = "Approved", squire = 2, squireName = "Gawain", amount = 5, questId = 1, questTitle = "Make your bed", actorName = "Dad"),
            HistoryEntryDto(at = at - 180_000, kind = "Redeemed", squire = 2, squireName = "Gawain", amount = 3, itemId = 1, itemTitle = "Ice cream", actorName = "Dad"),
            HistoryEntryDto(at = at - 240_000, kind = "Adjusted", squire = 2, squireName = "Gawain", amount = 10, currency = Currency.Coins, reason = "gophering", actorName = "Dad"),
        )
        paparazzi.snapshot {
            SquireTheme {
                KnightHomeScreen(
                    state = KnightUiState.Ready(review, fromCache = false),
                    name = "David Storey",
                    initialTab = com.squire.knight.app.ui.KnightTab.History,
                    history = history,
                    onRefresh = {}, onApproveClaim = {}, onRejectClaim = { _, _ -> },
                    onApproveRequest = {}, onRejectRequest = { _, _ -> }, onAddFunds = { _, _, _, _ -> },
                    onPay = { _, _, _ -> },
                    onRedeem = { _, _ -> }, onMarkDone = { _, _, _ -> },
                )
            }
        }
    }

    @Test
    fun knightAddFunds() {
        // The "Add funds" dialog body — a grown-up can now grant Dollars, not just Coins (the currency
        // toggle). Rendered as a card (Paparazzi can't capture the AlertDialog popup).
        paparazzi.snapshot {
            SquireTheme {
                Surface(
                    modifier = Modifier.padding(24.dp),
                    shape = RoundedCornerShape(24.dp),
                    color = MaterialTheme.colorScheme.surface,
                    shadowElevation = 4.dp,
                ) {
                    Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                        Text("Add to Gawain", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
                        AddFundsFields(amount = "5", onAmount = {}, reason = "Birthday money", onReason = {}, currency = Currency.Cash, onCurrency = {})
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            TextButton(onClick = {}) { Text("Cancel") }
                            Button(onClick = {}) { Text("Add") }
                        }
                    }
                }
            }
        }
    }

    @Test
    fun squireQuestDetailAuthored() {
        // The description here is the exact blurb the integration test
        // `authored_quest_description_reaches_the_card` drives through create_quest → /state → the card,
        // so this render reflects a genuinely authored (not invented) description.
        val quest = QuestCard(
            on = 1, questId = 1L, reward = 5, status = QuestStatus.Available,
            title = "Tidy your room", icon = "🧹", category = "Chores",
            description = "Make your bed, put your clothes away, and clear the floor.",
        )
        paparazzi.snapshot {
            SquireTheme {
                Column(modifier = Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).padding(20.dp)) {
                    DetailCard(questDetail(quest), Modifier.fillMaxWidth())
                }
            }
        }
    }

    @Test
    fun squireSubmitFeedback() {
        // A turn-in used to pop a "Sent!" snackbar. Now the row itself answers (SQUIRE-T-0133): the
        // tapped quest shows ghost coins and an empty seal slot until the next view arrives — even
        // offline, where the store is not optimistic and the server status is still Available.
        val view = StateView(
            balance = 12, generatedAt = 0L, squire = 1L,
            questsToday = listOf(
                QuestCard(on = 1, questId = 1L, reward = 5, status = QuestStatus.Available, title = "Practice piano", icon = "🎹"),
                QuestCard(on = 1, questId = 2L, reward = 3, status = QuestStatus.Available, title = "Feed the dog", icon = "🐶"),
            ),
            rewards = emptyList(), myClaims = emptyList(), myRequests = emptyList(),
            streaks = emptyList(), badges = emptyList(), goals = emptyList(),
        )
        paparazzi.snapshot {
            SquireTheme {
                PlayerHomeScreen(
                    state = PlayerUiState.Ready(view, fromCache = true), // offline: the last saved view
                    onRefresh = {}, onMarkDone = {}, onRedeem = {},
                    headerLabel = "Matrim",
                    tappedQuestIds = setOf(1L), // seam: "Practice piano" was just tapped
                )
            }
        }
    }

    @Test
    fun squireRewardDetailVariants() {
        // The reward detail renders differently by state (SQUIRE-T-0094 #8): affordable, still-saving,
        // achievement-locked, and out-of-stock.
        val affordable = RewardCard(affordable = true, cost = 3, itemId = 1L, name = "Ice cream", icon = "🍦", description = "A scoop of your favourite ice cream after dinner.")
        val saving = RewardCard(affordable = false, cost = 15, itemId = 2L, name = "Movie night", icon = "🎬", description = "Pick a film and watch it together.")
        val locked = RewardCard(affordable = false, cost = 10, itemId = 3L, name = "Toy car", icon = "🚗", description = "A shiny new race car.", lock = LockReason(kind = LockReasonKind.NeedsAchievement, name = "Chore Champion"))
        val outOfStock = RewardCard(affordable = true, cost = 8, itemId = 4L, name = "Last cookie", icon = "🍪", description = "The very last cookie in the jar.", lock = LockReason(kind = LockReasonKind.OutOfStock))
        paparazzi.snapshot {
            SquireTheme {
                Column(
                    modifier = Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    DetailCard(rewardDetail(affordable), Modifier.fillMaxWidth())
                    DetailCard(rewardDetail(saving), Modifier.fillMaxWidth())
                    DetailCard(rewardDetail(locked), Modifier.fillMaxWidth())
                    DetailCard(rewardDetail(outOfStock), Modifier.fillMaxWidth())
                }
            }
        }
    }

    @Test
    fun squireDetailCards() {
        // The tap-to-expand detail views (SQUIRE-T-0094 #8), one per card type, rendered as cards
        // (Paparazzi can't capture the real AlertDialog popups).
        val quest = QuestCard(on = 1, questId = 1L, reward = 5, status = QuestStatus.Pending, title = "Tidy your room", icon = "🧹", category = "Chores", description = "Make your bed, put your clothes away, and clear the floor.")
        val reward = RewardCard(affordable = true, cost = 3, itemId = 1L, name = "Ice cream", icon = "🍦", description = "A scoop of your favourite ice cream after dinner.")
        val streak = StreakView(name = "Room Master", current = 3, best = 5, alive = true, nextMilestone = 7)
        val badge = BadgeView(at = 0L, bonus = 25, id = 1L, name = "Century Club")
        val goal = GoalView(id = 7L, name = "Best Friends", description = "Complete 10 chores together", bonus = 25, current = 6, target = 10)
        paparazzi.snapshot {
            SquireTheme {
                Column(
                    modifier = Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    DetailCard(questDetail(quest), Modifier.fillMaxWidth())
                    DetailCard(rewardDetail(reward), Modifier.fillMaxWidth())
                    DetailCard(streakDetail(streak), Modifier.fillMaxWidth())
                    DetailCard(badgeDetail(badge), Modifier.fillMaxWidth())
                    DetailCard(goalDetail(goal), Modifier.fillMaxWidth())
                }
            }
        }
    }

    @Test
    fun squireGoalDetail() {
        // The goal detail on its own (SQUIRE-T-0122): "how to earn it" reads as a stacked, full-width
        // block above the stats, and a Progress line shows how close the child is.
        val goal = GoalView(id = 7L, name = "Best Friends", description = "Complete 10 chores together", bonus = 25, current = 6, target = 10)
        paparazzi.snapshot {
            SquireTheme {
                Column(
                    modifier = Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).padding(16.dp),
                ) {
                    DetailCard(goalDetail(goal), Modifier.fillMaxWidth())
                }
            }
        }
    }

    @Test
    fun squirePlayerHome() {
        val view = StateView(
            balance = 12,
            // Dual-currency header: coins (⭐, from `balance`) + real-money dollars owed ($, SQUIRE-T-0099).
            balances = listOf(CurrencyBalance(balance = 5, currency = Currency.Cash, name = "Dollars", symbol = "$")),
            generatedAt = 0L,
            squire = 1L,
            questsToday = listOf(
                QuestCard(on = 1, questId = 1L, reward = 5, status = QuestStatus.Available, title = "Make your bed", icon = "🛏"),
                QuestCard(on = 1, questId = 2L, reward = 10, status = QuestStatus.Pending, title = "Tidy your room", icon = "🧹"),
                QuestCard(on = 1, questId = 3L, reward = 3, status = QuestStatus.CompletedToday, title = "Feed the dog", icon = "🐶"),
                QuestCard(on = 1, questId = 4L, reward = 4, status = QuestStatus.Available, title = "Practice piano", icon = "🎹"),
            ),
            rewards = listOf(
                RewardCard(affordable = true, cost = 3, itemId = 1L, name = "Ice cream", icon = "🍦"),
                RewardCard(affordable = false, cost = 15, itemId = 2L, name = "Movie night", icon = "🎬"),
            ),
            // A "not yet" on today's piano turn-in (SQUIRE-T-0133): the reason shows on the card.
            myClaims = listOf(
                ClaimStatus(claimId = 9L, on = 1, questTitle = "Practice piano",
                    state = ClaimState(state = ClaimStateKind.Rejected, reason = "Play the whole piece through once more")),
            ),
            myRequests = emptyList(),
            streaks = listOf(StreakView(name = "Room Master", current = 3, best = 5, alive = true, nextMilestone = 7)),
            badges = emptyList(),
            goals = listOf(
                GoalView(id = 7L, name = "Best Friends", description = "Complete 10 chores together", bonus = 25, current = 6, target = 10),
                GoalView(id = 8L, name = "Century", description = "Earn 100 coins", bonus = 0, current = 40, target = 100),
            ),
        )
        // Wears the squire's tincture (SQUIRE-T-0136) exactly as the host does: theme from the view.
        val red = view.copy(tincture = "gules")
        paparazzi.snapshot {
            SquireTheme(tincture = red.tincture) {
                PlayerHomeScreen(
                    state = PlayerUiState.Ready(red, fromCache = false),
                    onRefresh = {}, onMarkDone = {}, onRedeem = {},
                    onPickTincture = {},
                    headerLabel = "Matrim Oakenfury", // realistic long name — stresses the header layout
                )
            }
        }
    }

    @Test
    fun squirePlayerRewards() {
        val view = StateView(
            balance = 12, generatedAt = 0L, squire = 1L,
            // Owed $5 → the "Cash out" affordance shows above the coin shop (SQUIRE-T-0118).
            balances = listOf(CurrencyBalance(balance = 5, currency = Currency.Cash, name = "Dollars", symbol = "$")),
            questsToday = emptyList(),
            rewards = listOf(
                RewardCard(affordable = true, cost = 3, itemId = 1L, name = "Ice cream", icon = "🍦"),
                RewardCard(affordable = true, cost = 8, itemId = 3L, name = "Extra screen time", icon = "📺"),
                RewardCard(affordable = false, cost = 15, itemId = 2L, name = "Movie night", icon = "🎬"),
            ),
            myClaims = emptyList(), myRequests = emptyList(),
            streaks = emptyList(), badges = emptyList(), goals = emptyList(),
        )
        paparazzi.snapshot {
            SquireTheme {
                PlayerHomeScreen(
                    state = PlayerUiState.Ready(view, fromCache = false),
                    onRefresh = {}, onMarkDone = {}, onRedeem = {},
                    initialTab = SquireTab.Rewards,
                )
            }
        }
    }

    @Test
    fun squirePlayerMe() {
        val view = StateView(
            balance = 12, generatedAt = 0L, squire = 1L,
            questsToday = emptyList(), rewards = emptyList(),
            myClaims = emptyList(), myRequests = emptyList(),
            badges = listOf(
                BadgeView(at = 0L, bonus = 25, id = 1L, name = "Century Club"),
                BadgeView(at = 0L, bonus = 50, id = 2L, name = "Chore Champion"),
            ),
            streaks = listOf(
                StreakView(name = "Room Master", current = 3, best = 5, alive = true, nextMilestone = 7),
                StreakView(name = "Early Bird", current = 1, best = 4, alive = true, nextMilestone = 3),
            ),
            goals = listOf(
                GoalView(id = 7L, name = "Best Friends", description = "Complete 10 chores together", bonus = 25, current = 6, target = 10),
            ),
        )
        paparazzi.snapshot {
            SquireTheme(tincture = "gules") {
                PlayerHomeScreen(
                    state = PlayerUiState.Ready(view.copy(tincture = "gules"), fromCache = false),
                    onPickTincture = {},
                    onRefresh = {}, onMarkDone = {}, onRedeem = {},
                    initialTab = SquireTab.Me,
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
    fun knightManageQuestsComposing() {
        // The regression this pins (SQUIRE-T-0143): the list-first change put the form BELOW the
        // list, so tapping Edit prefilled a form nobody could see and editing looked broken. The form
        // must render above the board whenever it is open.
        val adapter = KnightApiAdapter(baseUrl = "http://localhost", household = "demo", token = "t")
        val quests = listOf(
            QuestSummaryDto(id = 100, title = "Make your bed", reward = 5, category = "Bedroom",
                cadenceLabel = "Daily", assignmentLabel = "All squires", completion = CompletionDto.EachAssignee,
                repeatableWithinDay = false, autoApprove = true, active = true),
        )
        paparazzi.snapshot {
            SquireTheme {
                QuestAdminScreen(
                    adapter = adapter,
                    squires = listOf(2L to "Gawain"),
                    onBack = {},
                    initialQuests = quests,
                    libraryOverride = emptyList(),
                    initiallyComposing = true,
                )
            }
        }
    }

    @Test
    fun knightEditCatalog() {
        // The "Current rewards" list with the in-place Edit affordance (SQUIRE-T-0120/0126) — library
        // omitted so the list (and its Edit/Archive buttons) renders in the captured viewport.
        val adapter = KnightApiAdapter(baseUrl = "http://localhost", household = "demo", token = "t")
        val items = listOf(
            ItemSummaryDto(id = 1, name = "Extra screen time", cost = 10, summary = "Repeatable", active = true, availability = AvailabilityKind.Repeatable),
            ItemSummaryDto(id = 2, name = "Movie night pick", cost = 25, summary = "Once · needs: Century Club", active = true, availability = AvailabilityKind.Once, gate = 1),
        )
        paparazzi.snapshot {
            SquireTheme {
                RewardAdminScreen(
                    adapter = adapter,
                    onBack = {},
                    initialItems = items,
                    libraryOverride = emptyList(),
                    gateOverride = listOf(1L to "Century Club"),
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
            streaks = listOf(StreakView(name = "Room Master", current = 3, best = 5, alive = true, nextMilestone = 7)),
            badges = listOf(
                BadgeView(at = 0L, bonus = 25, id = 1L, name = "Century Club"),
                BadgeView(at = 0L, bonus = 50, id = 2L, name = "Chore Champion"),
            ),
            // One merged feed, newest-first across types — a coin grant interleaved between a claim
            // and a request, not grouped by type (SQUIRE-T-0124).
            recentActivity = listOf(
                ActivityEntry(at = 400, kind = ActivityKind.Claim,
                    claim = ClaimStatus(claimId = 12L, on = 1, questTitle = "Take out the trash", state = ClaimState(state = ClaimStateKind.Pending))),
                ActivityEntry(at = 300, kind = ActivityKind.Adjustment,
                    adjustment = AdjustmentView(amount = 10, reason = "gophering", at = 300)),
                ActivityEntry(at = 200, kind = ActivityKind.Request,
                    request = RedemptionStatus(cost = 25, itemName = "Movie night", requestId = 20L, state = RedemptionState(state = RedemptionStateKind.Rejected, reason = "After homework"))),
                ActivityEntry(at = 100, kind = ActivityKind.Claim,
                    claim = ClaimStatus(claimId = 10L, on = 1, questTitle = "Tidy your room", state = ClaimState(state = ClaimStateKind.Approved, points = 10))),
            ),
        )
        paparazzi.snapshot {
            SquireTheme {
                PlayerHomeScreen(
                    state = PlayerUiState.Ready(view, fromCache = false),
                    onRefresh = {}, onMarkDone = {}, onRedeem = {},
                    initialTab = SquireTab.Activity, // the compact recent-activity page
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
