# Spec Delta

## MODIFIED Requirements

### Requirement: Large results spill to disk

The system SHALL return at most 4 KB inline from a command, job wait, or fetch result, SHALL write larger output to a file under the session's output directory, and SHALL include that path, the original byte count, and a hint to read or search it. It SHALL NOT spill a `read` result: a long read returns a head and tail with a hint to use offset and limit. It SHALL allow `read` to open files inside the output directory, and SHALL delete output files older than seven days when a session starts.

#### Scenario: Big command output
- **WHEN** a command writes 1 MB to stdout
- **THEN** the result holds a short head and tail, the spill path, and the original size

#### Scenario: Spill file readable
- **WHEN** the model reads the spill path from a previous result
- **THEN** `read` returns the content instead of refusing it as outside the session

#### Scenario: Job log readable
- **WHEN** a job writes a log under the output directory
- **THEN** `read` can open that log

#### Scenario: Read does not spill
- **WHEN** a `read` result exceeds 4 KB
- **THEN** the result holds a head and tail with a hint to use offset and limit, and no new output file is created

#### Scenario: Old output removed
- **WHEN** a session starts and an output file is eight days old
- **THEN** the file is deleted

#### Scenario: Recent output kept
- **WHEN** a session starts and an output file is one day old
- **THEN** the file stays
