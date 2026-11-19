# Spec Delta

## REMOVED Requirements

### Requirement: Large results spill to disk

**Reason**: command and fetch output belongs in a bounded memory buffer, not a file with no cleanup, and the disk path was unreadable through the jail.

**Migration**: the buffered-results requirement below replaces it. Job logs stay on disk because a background job can be unbounded and outlive a turn.

## ADDED Requirements

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
- **THEN** all buffers are cleared and old addresses report expiry

### Requirement: Long reads are trimmed

The system SHALL NOT buffer a `read` result: a read longer than 4 KB SHALL return a head and a tail with a hint to use offset and limit, and SHALL create no buffer.

#### Scenario: Read does not buffer
- **WHEN** a `read` result exceeds 4 KB
- **THEN** the result holds a head and tail with a hint to use offset and limit, and no buffer is created

### Requirement: Job logs stay readable

The system SHALL write background job output to a file under the output directory, SHALL allow `read` to open files inside that directory, and SHALL delete job log files older than seven days when a session starts.

#### Scenario: Job log readable
- **WHEN** the model reads a job log path from a previous result
- **THEN** `read` returns the content instead of refusing it as outside the session

#### Scenario: Old log removed
- **WHEN** a session starts and a job log is eight days old
- **THEN** the file is deleted

#### Scenario: Recent log kept
- **WHEN** a session starts and a job log is one day old
- **THEN** the file stays
