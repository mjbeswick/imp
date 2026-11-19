# Spec Delta

## ADDED Requirements

### Requirement: Tool rows share one indent and wrap their content

The system SHALL start every tool row with its mark in column one, SHALL place the summary after one space, SHALL indent wrapped continuations, failure previews, questions, and options by two columns, SHALL keep long summaries whole and wrap them to the terminal width, and SHALL NOT truncate a summary or preview to one line for width.

#### Scenario: Success and failure align
- **WHEN** a successful row and a failed row sit next to each other
- **THEN** both marks are in column one and both summaries start in column three

#### Scenario: Long command
- **WHEN** a bash command is 300 characters wide
- **THEN** it wraps across rows at the two-column indent, and the duration stays at the end of the summary

#### Scenario: Failure preview
- **WHEN** a command times out and its reason is longer than the terminal
- **THEN** the reason wraps at the two-column indent instead of being clipped

#### Scenario: Ask question
- **WHEN** an ask row carries a question and options wider than the terminal
- **THEN** the question and each option wrap beneath the action at the two-column indent

#### Scenario: No width ellipsis
- **WHEN** a row is wider than the terminal
- **THEN** it wraps, and the only ellipsis allowed marks content the model itself shortened
