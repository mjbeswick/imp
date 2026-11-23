## ADDED Requirements

### Requirement: The provider form moves between fields

The system SHALL let the user move between the fields of the provider form opened by `/provider add`, `/provider edit`, or first-run setup, SHALL move to the previous field on ↑ or Shift+Tab and to the next field on ↓, SHALL keep each field's text when moving, SHALL stop at the first and last field, SHALL advance on Enter and save from the last field, and SHALL cancel the whole form on Esc.

#### Scenario: Fix an earlier field
- **WHEN** the user has typed `model` and is on `key_env`, and presses ↑ twice and types a new model name
- **THEN** the model field holds the new name, the other fields keep their text, and nothing is saved yet

#### Scenario: Save from the last field
- **WHEN** the user is on `key_env` and presses Enter
- **THEN** the provider is saved with every field as it now reads

#### Scenario: Cancel
- **WHEN** the user presses Esc at any field
- **THEN** the form closes, nothing is saved, and the transcript says `provider cancelled`

#### Scenario: Bounds
- **WHEN** the user presses ↑ on `base` or ↓ on `key_env`
- **THEN** the cursor stays where it is
