## ADDED Requirements

### Requirement: Context window table for built-in models

The system SHALL show the context percentage in the status line for the built-in models `gpt-4.1-mini` and
`gemini-2.5-pro`, using the published window of each model, and SHALL let a configured `context_window`
override the table.

#### Scenario: OpenAI default model shows a percentage
- **WHEN** the session model is `gpt-4.1-mini` and no `context_window` is set
- **THEN** the status line shows the percentage of 1,047,576 tokens in use

#### Scenario: Gemini default model shows a percentage
- **WHEN** the session model is `gemini-2.5-pro` and no `context_window` is set
- **THEN** the status line shows the percentage of 1,048,576 tokens in use

#### Scenario: Configured window wins
- **WHEN** the session model is `gpt-4.1-mini` and `context_window` is set to 128000
- **THEN** the percentage is computed against 128000 tokens

#### Scenario: Unlisted model
- **WHEN** the session model has no table entry and no `context_window` is set
- **THEN** the status line shows no percentage, as before
