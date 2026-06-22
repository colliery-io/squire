Feature: Completion claims and Knight review (the credit flow)
  A Squire submits a completion claim for a quest. It stays pending until a Knight reviews it; on
  approval the quest's reward credits the Squire's coin balance (the AC-1 claim→review→credit path).

  Background:
    Given a fresh household with a Knight "Arthur"
    And a Squire "Lancelot"
    And a daily quest "Tidy room" worth 5 coins

  Scenario: A submitted claim is pending and credits nothing yet
    When Lancelot submits a claim for "Tidy room"
    Then the claim is pending
    And Lancelot's coin balance is 0

  Scenario: Approving a claim credits the reward
    When Lancelot submits a claim for "Tidy room"
    And the Knight approves the claim
    Then the request succeeds
    And Lancelot's coin balance is 5

  Scenario: An approved quest drops off the active list (usage feedback)
    When Lancelot submits a claim for "Tidy room"
    Then Lancelot's active quests include "Tidy room"
    When the Knight approves the claim
    Then Lancelot's active quests do not include "Tidy room"
