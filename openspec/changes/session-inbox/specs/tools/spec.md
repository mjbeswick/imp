# Spec Delta

## ADDED Requirements

### Requirement: Live sessions share one buffer budget

The system SHALL apply the 16 MB memory and 64 MB spill limits for output buffers across all live sessions together, SHALL keep each session's addresses separate, and SHALL drop a closed session's buffers.

#### Scenario: Shared memory
- **WHEN** two live sessions hold 10 MB of buffers each
- **THEN** the least recently used buffers move to spill until memory holds at most 16 MB

#### Scenario: Separate addresses
- **WHEN** one session reads `imp://out/1` from another session
- **THEN** the result reports the address as unknown

#### Scenario: Closed session
- **WHEN** the user closes a session in the inbox
- **THEN** its buffers are freed
