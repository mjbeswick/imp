# Spec Delta

## ADDED Requirements

### Requirement: Sessions can be forked

The system SHALL copy the running session's transcript and sidecar into a new session when `/fork` runs, SHALL give the copy a new id and a title naming the source, SHALL leave the source transcript and sidecar unchanged, SHALL attach the copy to the running session so later turns append to it, and SHALL report both ids.

#### Scenario: Fork copies the history
- **WHEN** the user runs `/fork` after several turns
- **THEN** a new transcript exists with the same bytes as the source and the session continues in the copy

#### Scenario: Source survives
- **WHEN** the fork exists
- **THEN** the original session still appears in `/sessions` with its own history unchanged

#### Scenario: Title
- **WHEN** the source is titled "flaky rsync test"
- **THEN** the fork is titled "fork of flaky rsync test"

#### Scenario: Exit id
- **WHEN** the user quits after forking
- **THEN** the printed resume id is the fork's id, not the source's

#### Scenario: Line mode
- **WHEN** the user runs `/fork` in line mode
- **THEN** both ids print and the next message appends to the fork

#### Scenario: No session yet
- **WHEN** `/fork` runs before a session log exists
- **THEN** the transcript reports that there is nothing to fork and nothing changes
