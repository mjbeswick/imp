## ADDED Requirements

### Requirement: Background sessions raise toasts

The system SHALL show a toast on the status line when a session out of view finishes a turn, fails, asks a question, asks to confirm a command, wakes on a scheduled check-in, or reports a finished background job, SHALL name the session by its position and title in the form `2/3 fix the parser: needs an answer`, SHALL show the newest toast in place of an older one for three seconds, SHALL ring the bell as before, and SHALL leave the current session's own events in the transcript without a toast.

#### Scenario: Background reply
- **WHEN** session 2 of 3 finishes its turn while session 1 is in view
- **THEN** the status line reads `2/3 fix the parser: replied` for three seconds

#### Scenario: Background question
- **WHEN** session 2 of 3 asks a question while session 1 is in view
- **THEN** the status line reads `2/3 fix the parser: needs an answer`

#### Scenario: Current session
- **WHEN** the session in view finishes its turn
- **THEN** no toast is shown and the transcript carries the usual row

#### Scenario: Newest wins
- **WHEN** two background sessions finish within three seconds
- **THEN** the status line shows the second toast only

#### Scenario: Background job
- **WHEN** a background job of session 2 finishes while session 1 is in view
- **THEN** the status line reads `2/3 fix the parser: Background job 4 finished with exit 0; output: …`, cut to the row

### Requirement: Line mode prints job notices

The system SHALL print each job completion as a plain line in line mode and SHALL keep the same text in the transcript of the full-screen chat.

#### Scenario: Line mode job
- **WHEN** a background job finishes during an interactive `--cli` session
- **THEN** a line reads `Background job 4 finished with exit 0; output: …`
