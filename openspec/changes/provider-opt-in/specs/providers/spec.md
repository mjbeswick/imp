## ADDED Requirements

### Requirement: Keyless built-in providers are listed when switched on

The system SHALL hide the built-in providers that need no API key (Ollama and Custom) from the provider and model lists until they are switched on with `provider.<name>.enabled = true`, SHALL always list the active provider, SHALL never list a provider switched off with `provider.<name>.enabled = false`, and SHALL keep listing a built-in that needs a key once its key is set.

#### Scenario: Hidden by default
- **WHEN** the user has no `provider.ollama.enabled` or `provider.custom.enabled` setting and opens `/provider` or `/model`
- **THEN** neither Ollama nor Custom is listed

#### Scenario: Switched on in the config
- **WHEN** the config contains `provider.custom.enabled = true` and the user opens `/provider` or `/model`
- **THEN** Custom is listed in both

#### Scenario: Active provider stays listed
- **WHEN** Ollama is the active provider and no setting switches it on
- **THEN** Ollama is still listed in `/provider` and `/model`

#### Scenario: Switched off
- **WHEN** the config contains `provider.custom.enabled = false`
- **THEN** Custom is not listed in `/provider` or `/model`, because the user switched it off

#### Scenario: Keyed built-in unchanged
- **WHEN** the user has not set an OpenAI key
- **THEN** OpenAI is not listed in `/model`, and `/provider` shows it with the "no key" label, as before
