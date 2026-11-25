## ADDED Requirements

### Requirement: /skill queues a skill for the next message

The system SHALL let the user queue an enabled skill with `/skill <name>`, SHALL send that skill's full
instructions to the model with the next message only, SHALL keep the user's typed text as the message shown
in the transcript, SHALL refuse a name that is not an enabled skill with an error line, and SHALL show a
usage line listing enabled skills for bare `/skill`.

#### Scenario: Queue an enabled skill
- **WHEN** the user types `/skill config` and then sends `check the file`
- **THEN** the model receives the config skill's instructions with the message, and the transcript shows only `check the file`

#### Scenario: Queued once
- **WHEN** the user sends a second message after the one that carried the skill
- **THEN** the second message does not carry the skill's instructions again

#### Scenario: Unknown skill
- **WHEN** the user types `/skill nosuch`
- **THEN** an error line says there is no enabled skill called `nosuch`, and nothing is queued

#### Scenario: Switched-off skill
- **WHEN** the user types `/skill` for a skill switched off with `/skills`
- **THEN** an error line says the skill is switched off, and nothing is queued

#### Scenario: Bare command
- **WHEN** the user types `/skill` with no name
- **THEN** a usage line shows `/skill <name>` and lists the enabled skills
