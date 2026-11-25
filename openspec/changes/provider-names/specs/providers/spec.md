## ADDED Requirements

### Requirement: Provider profile names

The system SHALL give each new provider profile a default name taken from its dialect, with a numeric suffix when that name is taken, SHALL let the user edit the name in the provider form, and SHALL refuse a name that matches a built-in provider or another saved profile.

#### Scenario: Default name for a new profile
- **WHEN** the user adds a provider with dialect `openai` and no saved profile is named `openai-2`
- **THEN** the form's name field is pre-filled with `openai-2`, because every dialect name is a built-in name and cannot be used

#### Scenario: Default name with a taken name
- **WHEN** a profile named `custom-2` already exists and the user adds another provider with dialect `custom`
- **THEN** the form's name field is pre-filled with `custom-3`

#### Scenario: Built-in copy
- **WHEN** the user configures the built-in `deepseek` provider and saves it as a copy
- **THEN** the form's name field is pre-filled with `deepseek-2`, because `deepseek` is a built-in name and cannot be used

#### Scenario: Refuse a built-in name
- **WHEN** the user saves a profile named `openai`
- **THEN** an error row says the name is a built-in provider name, the form stays open, and nothing is saved

#### Scenario: Refuse a duplicate name
- **WHEN** the user saves a profile named `work` while another saved profile is named `work`
- **THEN** an error row says the provider already exists, the form stays open, and nothing is saved

#### Scenario: Keep its own name
- **WHEN** the user edits a saved profile `work` and saves it with the name `work` unchanged
- **THEN** the save succeeds

#### Scenario: Rename a profile
- **WHEN** the user edits a saved profile `work`, changes the name to `office`, and saves
- **THEN** the profile is saved as `office`, and `work` is no longer listed

#### Scenario: Invalid characters
- **WHEN** the user saves a name with a space or a slash
- **THEN** an error row says the name must be letters, digits, `-` or `_`, and the form stays open
