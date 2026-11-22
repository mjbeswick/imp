## ADDED Requirements

### Requirement: Every listed key acts

The system SHALL handle every key the keys card lists and every action the palette lists, each in the state the card names, and SHALL be checked by a test that fails when a listed key has no effect or no mapping.

#### Scenario: Dead palette row
- **WHEN** a palette action replays a key that nothing handles
- **THEN** the test fails and names the action

#### Scenario: New keys card row
- **WHEN** a row is added to the keys card without a key mapping
- **THEN** the test fails and names the row
