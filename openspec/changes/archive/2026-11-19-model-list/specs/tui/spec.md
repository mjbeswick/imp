# Spec Delta

## MODIFIED Requirements

### Requirement: The session model can be chosen

The system SHALL set the session model for `/model <name>`, SHALL open a picker for bare `/model` listing the provider's models, the current model, the provider default, and recents, SHALL fetch that list from the provider and fall back to a built-in catalog when the request fails, SHALL record a chosen model in recents, and SHALL NOT rewrite the config file.

#### Scenario: Direct
- **WHEN** the user runs `/model deepseek-v4-pro`
- **THEN** the next request uses that model

#### Scenario: Picker
- **WHEN** the user runs `/model` with no argument
- **THEN** the picker lists the provider's models, the current model, the default, and recents

#### Scenario: Live list
- **WHEN** the provider reports four models and `/model` opens
- **THEN** all four appear, not only the current model

#### Scenario: Fetch fails
- **WHEN** the models endpoint is unreachable or rejects the key
- **THEN** the picker keeps the catalog entries and the current model

#### Scenario: Filter
- **WHEN** the user types `flash` in the model picker
- **THEN** only names containing `flash` remain, from either source

#### Scenario: Recents
- **WHEN** a model is chosen
- **THEN** it appears at the front of the next picker
