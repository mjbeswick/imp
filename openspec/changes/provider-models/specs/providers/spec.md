## ADDED Requirements

### Requirement: A provider lists its models in the config

The system SHALL read `provider.<name>.models` as a comma-separated list of model names for a named provider, SHALL always include the provider's default model in that list, and SHALL write the list back to the managed providers block, omitting the line when the list is empty.

#### Scenario: Two models
- **WHEN** the config holds `provider.work.models = m1, m2`
- **THEN** the provider `work` offers `m1` and `m2`, and its default model is offered too

#### Scenario: Empty list
- **WHEN** a provider has no models beyond its default
- **THEN** the managed block has no `models` line for it
