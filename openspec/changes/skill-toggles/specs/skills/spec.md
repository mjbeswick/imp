# Skills

## ADDED Requirements

### Requirement: Skills can be enabled or disabled

The system SHALL read an enabled flag for each skill from `skill.<name>.enabled` in the config, SHALL treat a skill as enabled when the flag is absent or true, and SHALL treat it as disabled when the flag is false.

#### Scenario: Flag absent
- **WHEN** a config has no `skill.<name>.enabled` key
- **THEN** the skill is enabled

#### Scenario: Flag false
- **WHEN** the config sets `skill.<name>.enabled = false`
- **THEN** the skill is disabled

### Requirement: Disabled skills stay out of the session

A disabled skill SHALL NOT appear in the prompt index, and `skill_load` SHALL refuse it with a message that it is disabled.

#### Scenario: Index excludes disabled skill
- **WHEN** a session starts with one skill disabled
- **THEN** the prompt index does not list that skill

#### Scenario: Load refused
- **WHEN** the model calls `skill_load` for a disabled skill
- **THEN** the result says the skill is disabled and returns no body

### Requirement: The /skills picker toggles skills

The system SHALL provide a `/skills` command that lists each skill with its source and enabled state, SHALL toggle the selected skill on space or Enter, and SHALL save the new state to the config.

#### Scenario: Toggle a skill
- **WHEN** the user selects an enabled skill in `/skills` and presses space
- **THEN** the skill is disabled and `skill.<name>.enabled = false` is written to the config

#### Scenario: Reload applies the change
- **WHEN** the user runs `/reload` after a toggle
- **THEN** the running session reflects the saved state
