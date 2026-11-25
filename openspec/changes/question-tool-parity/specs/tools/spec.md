## MODIFIED Requirements

### Requirement: The agent can ask the user a question

The system SHALL provide an `ask` tool taking either one legacy question with optional two-to-six described choices and `multiple`, or an ordered nonempty `questions` array whose entries each have a question, optional short header, optional two-to-six described choices, and optional `multiple`. It SHALL block until all answers arrive, preserve the legacy string result, return ordered arrays of exact labels or custom text for a batch, and stop the turn if any question is dismissed.

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
- **THEN** a legacy call returns both labels joined by `, ` and a batch returns both as separate labels

#### Scenario: Described option
- **WHEN** an option has a description
- **THEN** the user sees it under the label and the model receives only the label

#### Scenario: Too many options
- **WHEN** the model supplies more than six choices
- **THEN** the call is rejected with a message asking for fewer

#### Scenario: Ordered batch
- **WHEN** the model asks two questions with different headers and the user answers each
- **THEN** the tool returns `{"answers": [ ["first choice"], ["second choice"] ]}` in question order

#### Scenario: Invalid batch
- **WHEN** the model supplies an empty questions array or an entry without a question
- **THEN** the call is rejected without opening the question interface

#### Scenario: Dismissed batch
- **WHEN** the user dismisses the first question in a batch and another tool call follows `ask` in the same response
- **THEN** the remaining questions and the following tool call do not run

### Requirement: Asking a yes/no question is one keystroke

The system SHALL render a question whose only choices are yes and no, each optionally suffixed with ` (Recommended)`, as a `y`/`n` prompt, SHALL return the chosen exact label, and SHALL treat `y` and `n` as typed text on any other question.

#### Scenario: Confirmation-style question
- **WHEN** the model asks a question with options `["Yes", "No"]`
- **THEN** pressing `y` answers Yes and pressing `n` answers No

#### Scenario: Recommended confirmation
- **WHEN** the choices are `Yes (Recommended)` and `No`
- **THEN** pressing `y` returns `Yes (Recommended)` and pressing `n` returns `No`

#### Scenario: Other two-choice question
- **WHEN** the options are `["Python", "Rust"]` and the user presses `y`
- **THEN** nothing is sent and the answer row reads `y`

#### Scenario: Escape
- **WHEN** the user presses Esc on any question
- **THEN** the result says the question was not answered
