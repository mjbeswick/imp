## ADDED Requirements

### Requirement: The baseline forbids narration between steps

The compiled baseline SHALL tell the model never to write text between tool calls.

#### Scenario: A multi-step task

- **WHEN** the baseline is composed
- **THEN** it contains the rule against text between tool calls
