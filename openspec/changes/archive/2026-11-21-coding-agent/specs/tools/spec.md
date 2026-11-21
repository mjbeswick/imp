## ADDED Requirements

### Requirement: Jail refusals do not ask

The system SHALL refuse a write or edit whose path the jail refuses without asking the user, SHALL return the jail's reason, and SHALL treat the scratch folder as inside the session directory for the write prompt.

#### Scenario: Outside path
- **WHEN** the model edits a file in another project in normal mode
- **THEN** no prompt appears and the result says the path is outside the session directory

#### Scenario: Scratch write
- **WHEN** the model writes `$IMP_TMP/notes.txt` in normal mode
- **THEN** no prompt appears

### Requirement: Cat counts as a read

The system SHALL treat a file printed whole by a safe `cat`, with at most `-n` and plain paths, as read once the command exits 0, so a later write may overwrite it.

#### Scenario: Cat then write
- **WHEN** the model runs `cat -n README.md` and then writes README.md
- **THEN** the write is not refused for an unread file
