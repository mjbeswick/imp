## MODIFIED Requirements

### Requirement: One command creates the layout

The system SHALL provide `imp init`, which creates the config directory, the state directories, and both parent directories for skills, agents, memory, sessions, and spilled output, and prints each path with `created` or `kept`.

#### Scenario: Fresh machine
- **WHEN** the user runs `imp init` with no config present
- **THEN** the directories and the example skill are created and each is reported

#### Scenario: Second run
- **WHEN** the user runs it again
- **THEN** nothing is overwritten; every existing path is reported as `kept`

### Requirement: Only two files are written

The system SHALL write exactly one skill example and a commented config template containing no API key, and SHALL NOT write any session, memory note, or agent: the built-in agents serve as examples.

#### Scenario: Template contents
- **WHEN** the config template is created
- **THEN** every meaningful key is present and commented, including `links`, and `api_key` is absent

#### Scenario: Examples are real
- **WHEN** the example skill is read
- **THEN** it is a valid skill and would load without edits

#### Scenario: No agent file
- **WHEN** `imp init` runs on a fresh machine
- **THEN** the agents directory is created empty
