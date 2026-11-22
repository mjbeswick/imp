## ADDED Requirements

### Requirement: Steering joins a running turn

The system SHALL, before each model request in a turn, add any messages the user steered in as user messages, in the order sent.

#### Scenario: Between steps
- **WHEN** the user steers a message while a tool runs
- **THEN** the next request after that tool includes it, and the turn does not restart
