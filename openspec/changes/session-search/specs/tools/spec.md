## ADDED Requirements

### Requirement: Background jobs end with their session

The system SHALL stop every running background job of a session when that session's worker ends, whether it closes to make room or imp exits, by signalling the job's process group.

#### Scenario: Closed for room
- **WHEN** a session with a running background job closes to make room for a fifth
- **THEN** the job's process group receives SIGTERM

#### Scenario: Finished jobs
- **WHEN** a session whose jobs have all exited closes
- **THEN** nothing is signalled
