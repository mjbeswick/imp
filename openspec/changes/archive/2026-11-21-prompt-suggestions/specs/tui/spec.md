## ADDED Requirements

### Requirement: Suggestions are accepted with one key

The system SHALL draw a suggestion greyed out in an empty input, SHALL fill the input with it on Tab or Right, SHALL drop it as soon as the user types, sends, or a turn starts, and SHALL NOT send it on Enter alone.

#### Scenario: Accept
- **WHEN** the input is empty, `run the tests` is suggested, and the user presses Tab then Enter
- **THEN** `run the tests` is sent

#### Scenario: Typed over
- **WHEN** a suggestion is shown and the user types `n`
- **THEN** the input holds `n` and the suggestion is gone

#### Scenario: Late arrival
- **WHEN** a suggestion arrives after the user has started typing
- **THEN** it is not shown
