## ADDED Requirements

### Requirement: The prompt names the session directory

The system SHALL name the session directory's absolute path in the system prompt and SHALL say that commands start there.

#### Scenario: Named
- **WHEN** a session starts in `/work/shop`
- **THEN** the system prompt contains `/work/shop`
