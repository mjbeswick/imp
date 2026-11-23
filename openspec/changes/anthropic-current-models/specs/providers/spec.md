## ADDED Requirements

### Requirement: Anthropic requests carry no sampling parameters

The system SHALL send no `temperature`, `top_p`, or `top_k` in requests to the Anthropic provider, whatever the model, and SHALL list current Claude models in the built-in catalog, with `claude-opus-5-5` as the provider default.

#### Scenario: Current model
- **WHEN** the session model is `claude-opus-5-5` and a turn starts
- **THEN** the request body has no `temperature`

#### Scenario: Catalog
- **WHEN** the models endpoint is unreachable and the user opens `/model` on Anthropic
- **THEN** the picker lists `claude-opus-5-5`, `claude-sonnet-5-5`, and `claude-haiku-5-5`
