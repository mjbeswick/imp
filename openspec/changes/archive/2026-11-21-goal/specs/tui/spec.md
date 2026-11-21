## ADDED Requirements

### Requirement: Goals are set with /goal

The system SHALL set a goal and start its first round on `/goal <text>`, SHALL show the goal and its round on `/goal`, SHALL clear it on `/goal off` or Esc or a sent message, and SHALL show `goal n/N` in the status line while one runs.

#### Scenario: Start
- **WHEN** the user types `/goal all tests pass`
- **THEN** a turn starts on it and the status line shows `goal 1/5`

#### Scenario: Esc
- **WHEN** the user presses Esc during a goal round
- **THEN** the round is cancelled and no further round starts
