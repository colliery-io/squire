Feature: Knight balance adjustments
  A Knight can grant or correct a Squire's balance. Coins are the default currency, and — the
  regression behind SQUIRE-T-0109 — the phone SDK serialises an explicit "currency": null, which
  the server must accept as Coins rather than reject. A reason is mandatory and only Knights may adjust.

  Background:
    Given a fresh household with a Knight "Arthur"
    And a Squire "Lancelot"

  Scenario: A Knight grants coins to a Squire
    When the Knight adjusts Lancelot by 5 coins with reason "good job"
    Then the request succeeds
    And Lancelot's coin balance is 5

  Scenario: An explicit null currency is accepted as Coins (T-0109 regression)
    When the Knight posts an adjust for Lancelot with amount 7, reason "gophering", and currency null
    Then the request succeeds
    And Lancelot's coin balance is 7

  Scenario: A blank reason is rejected
    When the Knight posts an adjust for Lancelot with amount 5, reason "", and currency null
    Then the request is rejected with status 400

  Scenario: A Squire cannot adjust balances
    When Lancelot posts an adjust for Lancelot with amount 5, reason "sneaky", and currency null
    Then the request is rejected with status 403
