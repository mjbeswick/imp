## ADDED Requirements

### Requirement: A long first message is summarised once

The system SHALL, after the first turn of a session ends without error and `title_summary` allows it, send one request holding only the first user message and no tools, asking for a title of at most six words with a limit of 24 tokens, SHALL send it only when no command is waiting, SHALL send it at most once per session, and SHALL keep the cut title when it fails.

#### Scenario: One request
- **WHEN** the first message is 90 characters and its turn ends
- **THEN** one extra request is sent whose body holds that message and no history, and the title updates

#### Scenario: Short message
- **WHEN** the first message is 24 characters
- **THEN** no title request is sent

#### Scenario: Busy
- **WHEN** the user has already sent the next message
- **THEN** no title request is sent and that message is not delayed

#### Scenario: Failure
- **WHEN** the provider answers 429
- **THEN** the cut title stays and no error row is shown
