# Providers

## ADDED Requirements

### Requirement: The providers list shows every provider and its state

The system SHALL list every built-in and named provider in the providers list modal, opened by `/provider`, marking each as active, unconfigured, or off.

#### Scenario: Listing
- **WHEN** the user opens `/provider`
- **THEN** every built-in and named provider appears, with its state marked

### Requirement: Enter opens a provider's config

The system SHALL open the provider config modal for the selected provider when Enter is pressed in the providers list.

#### Scenario: Open config
- **WHEN** the user selects a provider and presses Enter
- **THEN** the provider config modal opens for that provider

### Requirement: Keys act on the selected provider

The system SHALL make the selected provider active on `u`, toggle its enabled state on space, delete it on `d` when it is named, and open a new provider config on `n`.

#### Scenario: Toggle
- **WHEN** the user presses space on an enabled provider
- **THEN** the provider is disabled and `provider.<name>.enabled = false` is saved

#### Scenario: Built-in cannot be deleted
- **WHEN** the user presses `d` on a built-in provider
- **THEN** nothing is deleted and the user is told it can only be disabled

#### Scenario: Delete named
- **WHEN** the user presses `d` on a named provider and confirms
- **THEN** its lines are removed from the config

### Requirement: The config modal saves or cancels

The system SHALL save the edited provider config when the user chooses Save, SHALL discard the edits when the user chooses Cancel, and SHALL store a typed key only in the keys file.

#### Scenario: Save
- **WHEN** the user edits a field and saves
- **THEN** the config holds the new value

#### Scenario: Cancel
- **WHEN** the user edits a field and cancels
- **THEN** the config and the keys file are unchanged

#### Scenario: Key stored separately
- **WHEN** the user types an API key and saves
- **THEN** the key is written to the keys file and not to the config

### Requirement: The model list stays a model list

The `/model` list SHALL list models only and SHALL NOT carry provider actions.

#### Scenario: Model list
- **WHEN** the user opens `/model`
- **THEN** each row names its provider and offers no enable, disable, or delete action
