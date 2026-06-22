Feature: Full-stack durability (phone → API → store)
  State written through the API is persisted in the store and survives a server restart — both the
  event log (balances) and the credentials (the Squire can sign in again). This is the full-stack
  integration path the phone depends on: a write isn't "done" until it survives a restart.

  Background:
    Given a fresh household with a Knight "Arthur"
    And a Squire "Lancelot"

  Scenario: A granted balance survives a server restart
    When the Knight adjusts Lancelot by 12 coins with reason "chores"
    And the server restarts
    Then Lancelot's coin balance is 12

  Scenario: An approved claim's credit survives a server restart
    Given a daily quest "Tidy room" worth 5 coins
    When Lancelot submits a claim for "Tidy room"
    And the Knight approves the claim
    And the server restarts
    Then Lancelot's coin balance is 5
