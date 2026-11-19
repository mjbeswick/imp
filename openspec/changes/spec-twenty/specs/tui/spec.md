# Spec Delta

## MODIFIED Requirements

### Requirement: Nineteen slash commands, no more

The system SHALL provide exactly `/help`, `/new`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/yolo`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, `/shell`, `/model`, `/provider`, `/stash`, `/rename`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/mode`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all twenty commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Rename
- **WHEN** the user runs `/rename fix the parser`
- **THEN** the current session carries that title and no request is sent

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block
