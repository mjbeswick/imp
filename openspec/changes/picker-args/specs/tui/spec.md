# Spec Delta

## MODIFIED Requirements

### Requirement: Slash commands autocomplete

The system SHALL open the picker overlay when the input begins with `/`, SHALL show commands filtered by the word before the first space, SHALL rank prefix matches before substring matches, SHALL show a title and a footer naming Enter and Esc, SHALL accept the highlighted command with Tab or Enter, SHALL open the value picker after a command word and a space when that command has enumerable values, SHALL seed that picker's filter with the text after the space, and SHALL keep the picker closed for commands whose argument is free text.

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

#### Scenario: Values follow the command
- **WHEN** the user types `/theme ` and then `gru`
- **THEN** the theme picker opens with `gru` in the filter and `gruvbox-dark` listed

#### Scenario: Free text
- **WHEN** the user types `/stash remember the tests`
- **THEN** no picker opens and the whole text is stashed on Enter

## ADDED Requirements

### Requirement: Autocomplete rows show a description

The system SHALL render every command row as two columns, SHALL put the command name in the left column and its one-line description in the right column, SHALL align the right column across rows, SHALL never truncate the command name, SHALL clip a description that does not fit, SHALL apply the selection highlight across both columns, and SHALL match the filter against the command name only.

#### Scenario: Two columns
- **WHEN** the user types `/`
- **THEN** `/compact` sits on the left and `compact the conversation now` on the right

#### Scenario: Alignment
- **WHEN** rows of different name lengths are listed
- **THEN** every description starts at the same column

#### Scenario: Narrow terminal
- **WHEN** the box is too narrow for a description
- **THEN** the description is cut and the command name stays whole

#### Scenario: Filter
- **WHEN** the user types `/co`
- **THEN** matching uses the command name, not words in its description
