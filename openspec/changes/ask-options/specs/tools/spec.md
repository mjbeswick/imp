## MODIFIED Requirements

### Requirement: The agent can ask the user a question

The system SHALL provide an `ask` tool taking a question, optional two-to-six choices each with a label and an optional description, and an optional `multiple` flag, SHALL block until the user answers, SHALL return the chosen label, the marked labels joined by `, ` under `multiple`, or the user's typed answer, and SHALL stop the turn when the user dismisses the question.

#### Scenario: Multiple choice
- **WHEN** the model asks which of three deployment targets to use
- **THEN** the user picks one and the model receives that exact text as the tool result

#### Scenario: Custom answer
- **WHEN** the user ignores the offered choices and types their own answer
- **THEN** the model receives the typed text

#### Scenario: No options
- **WHEN** the model asks a question with no choices
- **THEN** the interface shows a free-text prompt and returns what the user types

#### Scenario: Declined
- **WHEN** the user dismisses the question
- **THEN** the tool result says the user did not answer and the turn stops

#### Scenario: Several choices
- **WHEN** the model asks with `multiple: true` and the user marks the first and third options
- **THEN** the model receives both labels joined by `, `

#### Scenario: Described option
- **WHEN** an option has a description
- **THEN** the user sees it under the label and the model receives only the label

#### Scenario: Too many options
- **WHEN** the model supplies more than six choices
- **THEN** the call is rejected with a message asking for fewer
