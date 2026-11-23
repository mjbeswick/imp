## ADDED Requirements

### Requirement: /goal pauses, resumes and clears a session's goal

The system SHALL accept `/goal pause`, `/goal resume`, and `/goal clear` in addition to `/goal <text>`, `/goal`, and `/goal off`, SHALL keep a paused goal's text and round, SHALL stop rounds while paused, and SHALL remove the goal on `clear` and `off`.

#### Scenario: Pause and resume
- **WHEN** a goal is at round 3 of 5 and the user runs `/goal pause` then `/goal resume`
- **THEN** the goal is paused at round 3 and then runs from round 3

#### Scenario: Clear
- **WHEN** the user runs `/goal clear` on a paused goal
- **THEN** the goal is removed and the status line no longer shows it

### Requirement: The status line shows the goal's state

The system SHALL show `goal n/m` in the status line while a goal runs and `goal paused` while it is paused.

#### Scenario: Running and paused
- **WHEN** a goal runs at round 2 of 5, then is paused
- **THEN** the status line shows `goal 2/5` and then `goal paused`
