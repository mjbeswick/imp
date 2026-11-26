## ADDED Requirements

### Requirement: The compact command takes focus text

The system SHALL run `/compact` with any text after it as the focus for that compaction, SHALL open no picker for that text, SHALL keep the typed text in the composer until Enter, and SHALL show the command's help line with the argument form `/compact [focus]`.

#### Scenario: Compact with focus
- **WHEN** the user types `/compact keep the migration plan` and presses Enter
- **THEN** the compaction runs with that focus and the composer clears

#### Scenario: Compact alone
- **WHEN** the user types `/compact` and presses Enter
- **THEN** the compaction runs with no focus

#### Scenario: No picker for focus text
- **WHEN** the user types `/compact keep` in the composer
- **THEN** no picker opens and the text stays in the composer

#### Scenario: Help line
- **WHEN** the user opens the help listing
- **THEN** the `/compact` line reads `/compact [focus]`
