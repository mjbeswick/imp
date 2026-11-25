## ADDED Requirements

### Requirement: Session marks for every session

The system SHALL save for each session whether it is waiting for an answer and whether it has an unread
reply, SHALL show `?` for a waiting session and `•` for an unread one in the session list whether or not
the session is open, SHALL clear the unread mark when the user views the session, and SHALL read sessions
saved before this change as not waiting and not unread.

#### Scenario: Waiting session that is not open
- **WHEN** a session asks a question, and the user has switched to another session
- **THEN** the session list shows `?` on that session's row

#### Scenario: Unread reply
- **WHEN** a reply finishes in a session that is not in view
- **THEN** the session list shows `•` on that row until the user views the session

#### Scenario: Cleared on view
- **WHEN** the user opens a session with an unread mark
- **THEN** the mark is cleared and the change is saved

#### Scenario: Marks survive a restart
- **WHEN** a session has an unread reply and imp is restarted
- **THEN** the session list still shows `•` on that session

#### Scenario: Older session files
- **WHEN** imp reads a session file saved before this change
- **THEN** the session shows no mark and opens without error

#### Scenario: Open session shows live state
- **WHEN** a session is open and its reply is streaming
- **THEN** the session row shows the streaming spinner, not a saved mark
