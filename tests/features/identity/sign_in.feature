@identity @A1 @A1.1
Feature: Sign in
  As a user of the attendance system
  I want to sign in with my email and password
  So that I can use the attendance features available to me

  # Prototype note: SUTD SSO and two-factor authentication are out of scope.
  # "Signed in" means the client holds a valid access token; the page steps of the app teams'
  # version of this story are not part of the API.
  # In tables, {space} stands for one space, because Gherkin trims table cells.

  Background:
    Given the API is running
    And an active account exists for "alex@example.edu" named "Alex Tan" with password "Correct#123"

  @happy @FR1 @FR2
  Scenario: Alex signs in with valid credentials
    When Alex signs in with email "alex@example.edu" and password "Correct#123"
    Then the response status is 200
    And the response contains an access token that expires in 24 hours

  @happy @FR9 @FR8
  Scenario: Alex views their own profile with the access token
    Given Alex has signed in
    When Alex requests their profile
    Then the response status is 200
    And the profile shows the email "alex@example.edu" and the name "Alex Tan"
    And the profile does not contain a password or a password hash

  @happy @FR5
  Scenario Outline: Alex's email is matched regardless of case and surrounding spaces
    When Alex signs in with email "<email>" and password "Correct#123"
    Then the response status is 200
    And the response contains an access token that expires in 24 hours

    Examples:
      | email                          |
      | ALEX@Example.edu               |
      | {space}alex@example.edu{space} |

  @sad @FR3
  Scenario Outline: A wrong password is rejected
    When Alex signs in with email "alex@example.edu" and password "<password>"
    Then the response status is 401
    And the error code is "invalid_credentials"
    And the error message is "Invalid email or password"
    And the response contains no access token

    Examples:
      | password     |
      | Wrong#999    |
      | correct#123  |

  @sad @FR4
  Scenario: An unknown email is rejected exactly like a wrong password
    When someone signs in with email "nobody@example.edu" and password "Correct#123"
    Then the response status is 401
    And the error code is "invalid_credentials"
    And the error message is "Invalid email or password"
    And the response contains no access token

  @sad @FR6
  Scenario Outline: A missing or blank email or password is rejected
    When someone signs in with email "<email>" and password "<password>"
    Then the response status is 400
    And the error code is "validation_failed"
    And the error details name the field "<field>"

    Examples:
      | email            | password     | field    |
      |                  | Correct#123  | email    |
      | {space}          | Correct#123  | email    |
      | alex@example.edu |              | password |
      | alex@example.edu | {space}      | password |

  @sad @FR7
  Scenario Outline: A correct password on an account that is not active is refused
    Given an account exists for "<email>" with password "Correct#123" and status "<status>"
    When someone signs in with email "<email>" and password "Correct#123"
    Then the response status is 403
    And the error code is "account_not_active"
    And the response contains no access token

    Examples:
      | email              | status    |
      | invited@example.edu | INVITED   |
      | banned@example.edu  | SUSPENDED |

  @sad @FR7
  Scenario: A wrong password on a suspended account does not reveal its status
    Given an account exists for "banned@example.edu" with password "Correct#123" and status "SUSPENDED"
    When someone signs in with email "banned@example.edu" and password "Wrong#999"
    Then the response status is 401
    And the error code is "invalid_credentials"

  @sad @FR10
  Scenario: A request without an access token is rejected
    When someone requests a profile without an access token
    Then the response status is 401
    And the error code is "unauthenticated"

  @sad @FR11
  Scenario Outline: A malformed or altered access token is rejected
    Given Alex has signed in
    When Alex requests their profile with a <kind> access token
    Then the response status is 401
    And the error code is "invalid_token"

    Examples:
      | kind      |
      | malformed |
      | altered   |

  @sad @FR12
  Scenario: An expired access token is rejected
    Given Alex has signed in
    And 24 hours have passed
    When Alex requests their profile
    Then the response status is 401
    And the error code is "token_expired"

  @sad @FR13
  Scenario: An access token stops working when the account is suspended
    Given Alex has signed in
    And Alex's account has been suspended
    When Alex requests their profile
    Then the response status is 403
    And the error code is "account_not_active"
