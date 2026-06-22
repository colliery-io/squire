Feature: Keep review queue (desktop)
  An operator (Knight) reviews completion claims and adjusts balances from the Keep web UI. The same
  review/adjust vocabulary the API Gherkin suite uses (SQUIRE-T-0110), expressed at the desktop.

  Background:
    Given the operator is signed in to the Keep
    And the Review tab is open

  # Adjust first, while Gawain is at his seeded balance — approving a claim below would credit him.
  Scenario: Adjusting a balance with a required reason
    When the operator adjusts "Gawain" by 10 with reason "Helped with dishes"
    Then "Gawain"'s coin balance shows 10

  Scenario: Rejecting a claim with an inline reason (no browser prompt)
    When the operator rejects the claim for "Tidy your room" with reason "Bed wasn't made"
    Then the claim for "Tidy your room" is no longer in the queue

  Scenario: Approving a claim clears it from the queue
    When the operator approves the claim for "Walk the dog"
    Then the claim for "Walk the dog" is no longer in the queue
