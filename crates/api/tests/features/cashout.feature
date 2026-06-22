Feature: Cashing out owed dollars (SQUIRE-T-0118)
  Owed real-world Cash is drawn down through the review queue, not a one-off "Pay" button: a Squire
  requests a cash-out, it stays pending until a Knight reviews it, and on approval the owed Cash is
  reduced. Rejecting leaves it owed. Mirrors the redemption flow, but for the Cash currency.

  Background:
    Given a fresh household with a Knight "Arthur"
    And a Squire "Lancelot"
    And the Knight grants Lancelot 5 dollars

  Scenario: A requested cash-out is pending and pays out nothing yet
    When Lancelot requests to cash out 5 dollars
    Then the request succeeds
    And Lancelot's owed cash is 5

  Scenario: Approving a cash-out draws down the owed Cash
    When Lancelot requests to cash out 5 dollars
    And the Knight approves the cash-out
    Then the request succeeds
    And Lancelot's owed cash is 0

  Scenario: Rejecting a cash-out leaves the Cash owed
    When Lancelot requests to cash out 5 dollars
    And the Knight rejects the cash-out
    Then the request succeeds
    And Lancelot's owed cash is 5

  Scenario: A Squire cannot cash out more than they are owed
    When Lancelot requests to cash out 8 dollars
    Then the request is rejected with status 409
