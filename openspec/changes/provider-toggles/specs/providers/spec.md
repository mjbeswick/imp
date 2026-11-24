# Providers

## ADDED Requirements

### Requirement: Built-in providers are listed only when configured

The system SHALL list a built-in provider in the `/model` list only when its key variable is set or it needs no key, SHALL NOT list an unconfigured built-in provider, and SHALL NOT require its key merely because it is listed.

#### Scenario: OpenAI without a key
- **WHEN** `OPENAI_API_KEY` is unset and OpenAI is not the active provider
- **THEN** no `openai::` row appears in the `/model` list

#### Scenario: Ollama needs no key
- **WHEN** the Ollama provider is not disabled
- **THEN** its rows appear in the `/model` list without a key

### Requirement: A provider can be switched off

The system SHALL read `provider.<name>.enabled` from the config, SHALL treat a provider as enabled when the flag is absent or true, and SHALL remove a provider set to false from the `/model` list.

#### Scenario: Switched off
- **WHEN** the config sets `provider.openai.enabled = false` and `OPENAI_API_KEY` is set
- **THEN** no `openai::` row appears in the `/model` list

#### Scenario: Active provider kept
- **WHEN** the active provider is switched off
- **THEN** its current model still appears, so the session is not hidden

### Requirement: The providers list shows each provider's state

The system SHALL show each provider in the providers list in one of three states: active, marked as current; unconfigured, dimmed with a note that no key is set; or disabled, struck through with the word off. A provider that is both unconfigured and disabled SHALL show as disabled.

#### Scenario: Active provider
- **WHEN** a provider is the active one
- **THEN** its row carries the current marker

#### Scenario: Unconfigured provider
- **WHEN** a provider needs a key and its key variable is unset
- **THEN** its row is dimmed and notes that no key is set

#### Scenario: Disabled provider
- **WHEN** `provider.<name>.enabled = false`
- **THEN** its row is struck through and marked off

### Requirement: Switching a provider off is saved

The system SHALL save `provider.<name>.enabled = false` to the config file, keeping its comments, and SHALL load it back as disabled.

#### Scenario: Round-trip
- **WHEN** the switch is saved and the config is loaded again
- **THEN** the provider is disabled
