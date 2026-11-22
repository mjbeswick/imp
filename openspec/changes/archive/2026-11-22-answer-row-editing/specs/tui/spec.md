## MODIFIED Requirements

### Requirement: Questions are answerable by key or by typing

The system SHALL render an `ask` request as a picker above the input line listing the choices with number keys, SHALL start a custom answer on a typed letter, kept as its first character, or on Space, empty, SHALL edit that answer with the composer's keys, SHALL show `type to answer   Esc stop` under the choices, SHALL ignore Enter on an empty answer, and SHALL leave the question unanswered and stop the turn on Esc.

#### Scenario: Pick by number
- **WHEN** the question offers three choices
- **THEN** pressing 2 selects the second and the answer is sent immediately

#### Scenario: Type an answer
- **WHEN** the user types a sentence on an open question and presses Enter
- **THEN** that sentence, first letter included, is the answer and it appears in the transcript

#### Scenario: Two choices that are not yes and no
- **WHEN** the choices are `Python` and `Rust` and the user types `yes`
- **THEN** the answer row reads `yes` and nothing is sent

#### Scenario: Empty answer
- **WHEN** the answer row is empty and the user presses Enter
- **THEN** nothing is sent and the row stays open

#### Scenario: Dismiss
- **WHEN** the user presses Esc on a question
- **THEN** the question closes, the tool result says it was not answered, the turn stops, and the input is ready

#### Scenario: Start with a digit
- **WHEN** the user presses Space on an open question, then types `3 files`
- **THEN** the answer row reads `3 files` and no choice is picked

#### Scenario: Edit the answer
- **WHEN** the answer row reads `use the old parser` and the user presses Ctrl+W
- **THEN** it reads `use the old `

#### Scenario: Paste into the answer
- **WHEN** imp holds a copy and the user presses Ctrl+V in the answer row
- **THEN** the copy is inserted at the caret
