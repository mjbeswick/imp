# Spec Delta

## MODIFIED Requirements

### Requirement: Slash commands autocomplete

The system SHALL open the picker overlay when the input begins with `/`, SHALL show matching commands filtered by the typed text, SHALL rank prefix matches before substring matches, SHALL show a title and a footer naming Enter and Esc, and SHALL accept the highlighted entry with Tab or Enter.

#### Scenario: Narrowing
- **WHEN** the user types `/co`
- **THEN** `/compact` is listed and other commands are filtered out

#### Scenario: Accept
- **WHEN** the user presses Tab
- **THEN** the highlighted command is completed in the input line and the overlay closes

#### Scenario: Run immediately
- **WHEN** the user presses Enter on the highlighted command
- **THEN** the command runs and the input clears

#### Scenario: Dismiss
- **WHEN** the user presses Esc
- **THEN** the overlay closes and the typed text stays
