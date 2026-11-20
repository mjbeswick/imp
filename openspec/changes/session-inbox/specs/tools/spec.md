# Spec Delta

## MODIFIED Requirements

### Requirement: Large results are buffered

The system SHALL return at most 4 KB inline from a command, job wait, or fetch result, SHALL keep the full output in a buffer addressed as `imp://out/<n>`, SHALL hold buffers in memory up to 16 MB, SHALL demote the least-recently-used buffer to an unlinked temporary file when memory is full, SHALL cap unlinked spill at 64 MB and drop the oldest file-backed buffer past it, SHALL include the address, the byte count, and a hint to read a range, and SHALL report an expired buffer by name.

#### Scenario: Big command output
- **WHEN** a command writes 1 MB to stdout
- **THEN** the result holds a short head and tail, an `imp://out/<n>` address, and the original size

#### Scenario: Read the buffer
- **WHEN** the model reads `imp://out/3` with offset and limit
- **THEN** it receives that slice with line numbers and no filesystem access

#### Scenario: Demotion keeps the id
- **WHEN** memory is full and the oldest buffer is demoted to a temporary file
- **THEN** reading its address still returns the text and no file appears in any directory

#### Scenario: Spill cap
- **WHEN** file-backed buffers exceed 64 MB
- **THEN** the oldest is dropped and its address reports expiry

#### Scenario: Expired buffer
- **WHEN** an address was dropped
- **THEN** the result says the buffer expired and names the address

#### Scenario: New session
- **WHEN** the user runs `/new`
- **THEN** the new session starts with no buffers, and the previous session keeps its own while it stays live

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
