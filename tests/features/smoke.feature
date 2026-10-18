@platform
Feature: Service health
  As an operator
  I want the service to answer a health check
  So that I know the deployed application is running

  Scenario: The API answers its health check
    Given the API is running
    When a client requests "GET /_ping"
    Then the response status is 200
