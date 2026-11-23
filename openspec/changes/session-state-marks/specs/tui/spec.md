## ADDED Requirements

### Requirement: The sessions modal lists every session and shows its state

The system SHALL list every saved session in the sessions modal and in search, including a session with no messages yet, shown as `(untitled)`, SHALL mark each row with `?` while that session waits for an answer or confirmation, a spinner while it runs, `!` when its last turn failed, and `•` when it finished and has not been viewed, in that order of priority, and SHALL leave the mark out for an idle session or a session not open in this window.

#### Scenario: Blank session
- **WHEN** a session was created with `/new` and never sent a message
- **THEN** the sessions modal lists it as `(untitled)` with `0 msgs`

#### Scenario: Running and waiting
- **WHEN** session 2 is running a turn and session 3 asks a question
- **THEN** session 2 shows the spinner and session 3 shows `?`

#### Scenario: Failed turn
- **WHEN** a session's last turn ends with an error
- **THEN** its row shows `!` until its next turn finishes without error

#### Scenario: Unseen reply
- **WHEN** a background session finishes a reply and the user has not viewed it
- **THEN** its row shows `•`

#### Scenario: Idle and closed
- **WHEN** a session is idle, or it is not open in this window
- **THEN** its row shows no mark
