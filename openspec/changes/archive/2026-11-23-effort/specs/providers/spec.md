## ADDED Requirements

### Requirement: Effort is a separate dial

The system SHALL keep an effort setting of `low`, `medium`, or `high`, unset by default, separate from the thinking switch, SHALL send it as `reasoning_effort` to OpenAI-compatible providers, sending `low` when unset, SHALL send it as `output_config.effort` to Anthropic when set, and SHALL note once per session when the provider has no such dial.

#### Scenario: Anthropic
- **WHEN** effort is `high` and the provider is Anthropic
- **THEN** the request body has `output_config.effort` set to `high`

#### Scenario: Unset
- **WHEN** effort is unset and the provider is DeepSeek with thinking on
- **THEN** the request sends `reasoning_effort: low`, as before

#### Scenario: No dial
- **WHEN** the provider is Ollama and the user runs `/effort high`
- **THEN** one line says the provider has no effort setting
