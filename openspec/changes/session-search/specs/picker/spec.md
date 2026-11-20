## ADDED Requirements

### Requirement: The session picker searches session text

The system SHALL, when the `/sessions` filter matches no title and the user presses Enter, search the message text of every stored session case-insensitively, SHALL list up to twenty matching sessions newest first with a snippet around the first hit, SHALL title the picker `search "<text>"`, and SHALL resume a session chosen from the results.

#### Scenario: Hit
- **WHEN** a session mentions `wttr.in`, no title contains it, and the user types `wttr` and presses Enter
- **THEN** the picker lists that session with a snippet containing `wttr.in`

#### Scenario: Case
- **WHEN** the user searches `WTTR`
- **THEN** the same session is listed

#### Scenario: No hit
- **WHEN** no session mentions the text
- **THEN** the picker shows the no-match row and the session is unchanged

#### Scenario: Resume from a hit
- **WHEN** the user presses Enter on a result
- **THEN** that session opens as it does from the session list

### Requirement: Working sessions spin in the session picker

The system SHALL mark each live session whose agent is working with a spinner before its message count in `/sessions`, SHALL animate it while the picker is open, and SHALL show no spinner on idle or stored sessions.

#### Scenario: Working
- **WHEN** a live session is streaming a reply and the user opens `/sessions`
- **THEN** its row's right column starts with a spinner frame and the frame advances

#### Scenario: Idle
- **WHEN** a live session is idle
- **THEN** its row shows only the message count
