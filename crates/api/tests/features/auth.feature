Feature: Role boundary (the trust boundary)
  A Squire token may read only its own state and may never reach a Knight-privileged route. Identity
  is token-derived, never a request parameter.

  Background:
    Given a fresh household with a Knight "Arthur"
    And a Squire "Lancelot"

  Scenario: A Squire cannot read the cross-household review
    When Lancelot requests the household review
    Then the request is rejected with status 403

  Scenario: A Squire can read its own state
    Then Lancelot's coin balance is 0
