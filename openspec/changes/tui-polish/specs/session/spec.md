# Spec Delta

## ADDED Requirements

### Requirement: Resume shows the restored transcript

The system SHALL draw a resumed session's user messages, assistant replies, and one finished row per tool call in the chat interface, SHALL omit reasoning and system messages, SHALL replace the previous transcript, SHALL show the restored context's token estimate in the status line, and SHALL count cost only for messages sent in the current process. This SHALL hold for `-r`, `-c`, and the session picker.

#### Scenario: Resume from the shell
- **WHEN** the user runs `imp -r <id>` for a session with three exchanges
- **THEN** the transcript shows the three user messages and replies in order and the status line shows a non-zero token count

#### Scenario: Resume from the picker
- **WHEN** the user picks a session while another conversation is on screen
- **THEN** the old transcript is replaced by the chosen session's

#### Scenario: Tool calls
- **WHEN** the resumed history holds a tool call
- **THEN** one finished tool row with its summary appears where the call was made

#### Scenario: Cost starts fresh
- **WHEN** a session is resumed and nothing is sent
- **THEN** the cost shown is zero

### Requirement: Session tables share one layout

The system SHALL print sessions in `--list`, line-mode `/sessions`, and the session picker with the same fields in the same order: id, age, title, and message count, SHALL show the title on one line by turning newlines into spaces, SHALL show `(untitled)` for an empty title, SHALL cut a title that does not fit and keep the count whole and aligned, and SHALL show the newest twenty in line-mode `/sessions` as the picker does.

#### Scenario: Line mode list
- **WHEN** the user runs `/sessions` in line mode with sixty sessions stored
- **THEN** twenty rows print with ages, and every count ends in one column

#### Scenario: Multi-line title
- **WHEN** a session title holds a newline
- **THEN** its row stays on one line

### Requirement: Session delete confirmation names the session

The system SHALL ask to delete a session by its title and id, and SHALL keep the session picker on screen behind the question.

#### Scenario: Delete from the picker
- **WHEN** the user presses `d` on `Rate my machine.`
- **THEN** the prompt reads `delete "Rate my machine." (51280befc)? y/N` and the list stays visible
