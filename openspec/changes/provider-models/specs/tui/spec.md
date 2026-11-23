## ADDED Requirements

### Requirement: The provider form takes a list of models

The system SHALL offer a `models` field in the provider form, SHALL split its value on commas into trimmed model names, and SHALL save them with the provider.

#### Scenario: Typed list
- **WHEN** the user types `m1, m2` into the `models` field and saves the form
- **THEN** the provider is saved with the models `m1` and `m2`

### Requirement: /model lists every configured provider's models

The system SHALL list, in the `/model` picker, every model of every built-in and named provider, SHALL show the provider beside each model, and SHALL switch the session to that provider and model when the user chooses the row.

#### Scenario: Model from another provider
- **WHEN** the session uses `deepseek` and the user chooses `m1` under the provider `work`
- **THEN** the session switches to `work` with the model `m1`, and the status line names `work`

#### Scenario: Same model under two providers
- **WHEN** `m1` is listed under `work` and under `ollama`
- **THEN** the picker shows two rows for `m1`, one per provider
