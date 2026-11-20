## ADDED Requirements

### Requirement: The idle loop sleeps

The system SHALL wait at most 50 ms between loop passes while a turn streams, a spinner turns, or within two seconds of a key or event, at most one second while a prompt waits, and at most five seconds otherwise, and SHALL wake at once on a key or a terminal resize.

#### Scenario: Idle
- **WHEN** nothing streams, nothing waits, and no key has arrived for two seconds
- **THEN** the loop waits up to five seconds

#### Scenario: Streaming
- **WHEN** a turn streams in any live session
- **THEN** the loop waits at most 50 ms

#### Scenario: Prompt
- **WHEN** an approval waits in view
- **THEN** the loop waits at most one second, so the prompt still expires on time

#### Scenario: Resize
- **WHEN** the terminal is resized while idle
- **THEN** the screen redraws at once
