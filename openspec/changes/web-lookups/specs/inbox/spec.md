## MODIFIED Requirements

### Requirement: Several sessions run at once

The system SHALL keep up to four live sessions in one process, each with its own worker, transcript, mode, approvals, and jobs, SHALL start another live session with `/new` and when resuming from `/sessions`, SHALL close the oldest idle session to make room when four are live, and SHALL refuse with a message when all four are working or waiting.

#### Scenario: Second session
- **WHEN** a turn is running and the user runs `/new`
- **THEN** a new empty session opens and the first keeps running

#### Scenario: Room made
- **WHEN** four sessions are live, one is idle, and the user runs `/new`
- **THEN** the idle one closes, stays in `/sessions`, and the new one opens

#### Scenario: No room
- **WHEN** four sessions are working or waiting and the user runs `/new`
- **THEN** the transcript says all four are busy and nothing closes

#### Scenario: Separate approvals
- **WHEN** the user answered `s` to `npm test` in one session
- **THEN** `npm test` in another session still asks
