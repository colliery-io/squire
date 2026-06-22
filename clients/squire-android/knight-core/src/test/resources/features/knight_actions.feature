Feature: Knight quick actions (Android offline store)
  The Knight app's offline-first store enqueues the correct command for each quick action — including
  the new cash payout (SQUIRE-T-0111), which must be a Cash adjustment of the negated amount, and the
  coin grant, which is a Coins adjustment. Both require a positive amount and a non-blank reason.

  Scenario: Paying a squire enqueues a Cash adjustment of the negated amount
    When the knight pays Gawain 5 dollars with reason "allowance"
    Then a Cash adjustment of -5 is submitted

  Scenario: Granting coins enqueues a Coins adjustment
    When the knight grants Gawain 10 coins with reason "good week"
    Then a Coins adjustment of 10 is submitted

  Scenario: A payout requires a positive amount and a reason
    Then paying Gawain 0 dollars with reason "nope" is rejected
    And paying Gawain 5 dollars with reason " " is rejected
