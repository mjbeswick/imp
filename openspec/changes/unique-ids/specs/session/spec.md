## ADDED Requirements

### Requirement: Session ids are unique

The system SHALL give every new or forked session an id no other stored session has, SHALL claim it atomically so two processes cannot take the same one, and SHALL add `-1`, `-2`, and so on when the time-based id is taken.

#### Scenario: Same second
- **WHEN** two sessions start within the same second
- **THEN** they have different ids and separate files

#### Scenario: Two processes
- **WHEN** two imp processes start at the same moment
- **THEN** neither writes into the other's session file
