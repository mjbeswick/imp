## ADDED Requirements

### Requirement: The baseline forbids narration between steps

The compiled baseline SHALL tell the model to write no text between tool calls and to keep text for the final reply or a question.

#### Scenario: A multi-step task

- **WHEN** the baseline is composed
- **THEN** it contains the rule against text between tool calls
