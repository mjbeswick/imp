# Spec Delta

## Purpose

Records each session on disk, lets the user start, continue, pick, rename, and delete sessions from the interface, and restores a resumed session with the exact context the model saw.

## MODIFIED Requirements

### Requirement: The user can start, continue, and pick sessions

The system SHALL start a new session by default, continue the most recent with `-c`, resume a named one with `-r <id>`, and open a picker in the interface that lists sessions newest first and starts a new one on request.

#### Scenario: Continue
- **WHEN** the user runs `imp -c`
- **THEN** the newest session is loaded and the input is ready

#### Scenario: Picker
- **WHEN** the user opens the session picker
- **THEN** sessions appear newest first with title, age, and message count, and choosing one loads it

#### Scenario: New from the picker
- **WHEN** the user chooses "new session" in the picker
- **THEN** a fresh session starts and the previous file stays intact

### Requirement: Sessions can be listed from the shell

The system SHALL provide `-l`/`--list` to print sessions newest first with id, title, age, and message count, and exit without starting the interface, and SHALL accept `--rename <id> <title>` and `--delete <id>` for scripted use.

#### Scenario: List and exit
- **WHEN** the user runs `imp --list`
- **THEN** a table of sessions prints and the process exits without opening the interface

#### Scenario: Scripted rename
- **WHEN** the user runs `imp --rename 20261117-1042 "flaky rsync"`
- **THEN** the sidecar title changes and the transcript is untouched

#### Scenario: Scripted delete
- **WHEN** the user runs `imp --delete 20261117-1042`
- **THEN** the session moves to trash after confirmation
