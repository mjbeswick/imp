## MODIFIED Requirements

### Requirement: Switch sessions by key

The system SHALL move to the next live session with Ctrl+N, wrapping at the end, SHALL keep each session's composer draft and scroll position when switching, and SHALL show the session's position and title in the title bar while more than one is live.

#### Scenario: Next session
- **WHEN** two sessions are live and the user presses Ctrl+N
- **THEN** the other session's transcript and draft appear, and the title bar reads `2/2` before its title

#### Scenario: One session
- **WHEN** only one session is live
- **THEN** Ctrl+N does nothing and the title bar shows no position
