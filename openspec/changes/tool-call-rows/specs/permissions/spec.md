# Spec Delta

## MODIFIED Requirements

### Requirement: The jail is independent of the mode

The system SHALL keep the path jail for file tools enabled in `normal` and `strict`, SHALL disable it only in `yolo`, SHALL keep imp's own configuration directory writable in every mode so skills, agents, and memory notes can be created, and SHALL state clearly when the jail is disabled.

#### Scenario: Yolo warning
- **WHEN** the session starts in `yolo`
- **THEN** the interface says the path jail is off

#### Scenario: Jail in normal mode
- **WHEN** the model writes outside the session directory in `normal` mode
- **THEN** it prompts, and a refusal means no write occurs

#### Scenario: Skills directory
- **WHEN** the model writes `~/.config/imp/skills/steno/SKILL.md` in `normal` mode
- **THEN** the write is allowed by the jail and still needs the usual confirmation

#### Scenario: Main config file
- **WHEN** the model writes `~/.config/imp/config` in `normal` mode
- **THEN** it is refused exactly as before, the directory exception does not cover it
