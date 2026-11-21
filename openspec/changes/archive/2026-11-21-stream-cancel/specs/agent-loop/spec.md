## MODIFIED Requirements

### Requirement: Cancellation is immediate

The system SHALL cancel the active turn within one second of the cancel key, SHALL abort an HTTP request in flight whether or not bytes are arriving, SHALL NOT retry a cancelled request, SHALL terminate any running tool process group within two seconds, and SHALL report the turn as cancelled.

#### Scenario: Cancel while streaming
- **WHEN** the user presses Esc while tokens are arriving
- **THEN** the stream stops, partial text remains visible, and the input is ready

#### Scenario: Cancel a running command
- **WHEN** the user presses Esc while a long `bash` command runs
- **THEN** the command's process group receives SIGTERM, then SIGKILL after two seconds, and the tool result reads cancelled

#### Scenario: Cancel while stalled
- **WHEN** the connection stops sending and the user cancels
- **THEN** the request ends within one second and the turn reads cancelled

#### Scenario: Not retried
- **WHEN** a request is cancelled
- **THEN** it is not sent again

## ADDED Requirements

### Requirement: A stalled stream ends

The system SHALL end a request that receives less than one byte a second for 60 seconds, and SHALL treat it as a transport error.

#### Scenario: Stall
- **WHEN** the provider stops sending mid-reply for a minute
- **THEN** the request ends with a transport error and is retried once
