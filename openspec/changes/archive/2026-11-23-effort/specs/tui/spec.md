## MODIFIED Requirements

### Requirement: Twenty-five slash commands, no more

The system SHALL provide exactly `/help`, `/keys`, `/diff`, `/new`, `/goal`, `/copy`, `/sessions`, `/agent`, `/reload`, `/compact`, `/retry`, `/think`, `/effort`, `/mode`, `/next`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, `/shell`, `/model`, `/provider`, `/queue`, `/rename`, and `/quit`, SHALL run them without sending a request to the model except `/retry`, and SHALL NOT provide `/clear`, `/yolo`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all twenty-seven commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Rename
- **WHEN** the user runs `/rename fix the parser`
- **THEN** the current session carries that title and no request is sent

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block

#### Scenario: Retry
- **WHEN** the user runs `/retry` after a finished reply
- **THEN** the last user message is sent again and the old reply leaves the transcript and the history

#### Scenario: Nothing to retry
- **WHEN** no message was sent, or a turn is running, and the user runs `/retry`
- **THEN** a line says `nothing to retry` or `a turn is running` and nothing is sent

#### Scenario: Effort
- **WHEN** the user runs `/effort high`
- **THEN** the next request asks the provider for high effort and the transcript says `effort: high`
