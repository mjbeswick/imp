## MODIFIED Requirements

### Requirement: Background sessions ask for attention

The system SHALL count sessions out of view that need approval or have replied and show the count in the status line, and SHALL ring the bell once when such a session first needs approval.

#### Scenario: Count
- **WHEN** two sessions out of view need approval
- **THEN** the status line shows `2 waiting`

#### Scenario: No deadline out of view
- **WHEN** a session out of view needs approval for ten minutes
- **THEN** the confirmation is still open and nothing ran

#### Scenario: Deadline in view
- **WHEN** the user switches to that session
- **THEN** the prompt shows, still open
