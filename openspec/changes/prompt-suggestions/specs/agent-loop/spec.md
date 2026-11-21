## ADDED Requirements

### Requirement: A suggestion follows each turn

The system SHALL, when `suggest` is on and a chat turn ends without error or cancel, make one request of at most 24 output tokens for the user's likely next message, SHALL NOT add it to the history or the session log, SHALL count its tokens in the cost, and SHALL drop a reply that is empty, multi-line, or over 60 characters.

#### Scenario: After a question
- **WHEN** a turn ends with the model asking whether to run the tests
- **THEN** a short suggestion such as `yes, run them` reaches the chat and the history is unchanged

#### Scenario: Off
- **WHEN** the config holds `suggest = off`
- **THEN** no suggestion request is made

#### Scenario: Line mode
- **WHEN** imp runs in line mode
- **THEN** no suggestion request is made
