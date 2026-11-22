## ADDED Requirements

### Requirement: A running turn takes steering and can be held

The system SHALL, before each model request in a turn, wait while the chat holds the turn, still honouring a cancel, and SHALL then add any steered messages as user messages in the order sent.

#### Scenario: Steered between steps
- **WHEN** the user steers a message while a tool runs
- **THEN** the next request after that tool includes it, and the turn does not restart

#### Scenario: Held while editing
- **WHEN** a queued message is open for editing when a tool finishes
- **THEN** no request is made until it is saved or left, and the status line shows `paused · editing queue`

#### Scenario: Stop while held
- **WHEN** the turn is held and the user presses Esc
- **THEN** the turn stops
