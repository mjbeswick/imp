## ADDED Requirements

### Requirement: Batch questions show progress

The TUI SHALL show each batch question's short header and its position in the batch, SHALL advance to the next after an answer, and SHALL dismiss the whole batch on Esc. Existing keyboard selection, free text, descriptions, and queued messages SHALL work on every question.

#### Scenario: Two questions
- **WHEN** a batch contains two questions
- **THEN** the first shows `1/2`, answering it shows `2/2`, and both answers are returned in order

#### Scenario: Custom answer after choices
- **WHEN** the user types a custom answer to the second question
- **THEN** that text is the second answer, not an option from the first question

#### Scenario: Dismiss in the middle
- **WHEN** the user presses Esc on the second question
- **THEN** the partial first answer is discarded and the turn stops

### Requirement: Line mode supports the question contract

Line mode SHALL show the header and descriptions for each question, accept one or more numbered choices under `multiple`, accept custom text, and return exact selected labels without discarding punctuation.

#### Scenario: Described multiple selection
- **WHEN** the user chooses the first and third described options in line mode
- **THEN** the result contains those two labels separately, without their descriptions

#### Scenario: Line-mode custom answer
- **WHEN** the user types text that is not an option number
- **THEN** that text is returned as the answer
