## ADDED Requirements

### Requirement: A check-in wakes its session

The system SHALL show a check-in in its session's transcript as an information line, SHALL mark the session working while the woken turn runs, and SHALL treat the turn's end as a reply, so a session out of view joins the queue.

#### Scenario: Out of view
- **WHEN** a check-in fires in a session out of view and its turn ends
- **THEN** the session is in the Ctrl+O queue and the status line counts it as waiting

#### Scenario: In view
- **WHEN** a check-in fires in the session in view
- **THEN** the check-in line appears and the reply streams below it
