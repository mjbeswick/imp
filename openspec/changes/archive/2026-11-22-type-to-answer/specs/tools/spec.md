## MODIFIED Requirements

### Requirement: Asking a yes/no question is one keystroke

The system SHALL render a question whose only choices are yes and no as a `y`/`n` prompt, SHALL return the chosen word, and SHALL treat `y` and `n` as typed text on any other question.

#### Scenario: Confirmation-style question
- **WHEN** the model asks a question with options `["Yes", "No"]`
- **THEN** pressing `y` answers Yes and pressing `n` answers No

#### Scenario: Other two-choice question
- **WHEN** the options are `["Python", "Rust"]` and the user presses `y`
- **THEN** nothing is sent and the answer row reads `y`

#### Scenario: Escape
- **WHEN** the user presses Esc on any question
- **THEN** the result says the question was not answered
