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
