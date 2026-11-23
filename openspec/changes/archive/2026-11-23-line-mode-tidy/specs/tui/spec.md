## MODIFIED Requirements

### Requirement: The app prints its session id on exit

The system SHALL print the current session id to stderr whenever an exit follows a session being opened, covering the full-screen chat, line mode, and a one-shot prompt, SHALL use the form `resume with imp -r <id>`, and SHALL print it only after the alternate screen, cursor, and cooked input are restored. It SHALL NOT print a session id for commands that open no session, such as `--help`, `--version`, `--list`, `--rename`, `--delete`, `init`, and `prompt`.

#### Scenario: Chat exit
- **WHEN** the user quits the full-screen chat with Ctrl+C
- **THEN** the primary screen shows one line naming the session id that `imp -r` accepts

#### Scenario: Line mode exit
- **WHEN** an interactive `--cli` session ends with `/quit` or end of input
- **THEN** the same resume line is printed

#### Scenario: One-shot exit
- **WHEN** `imp "why is the build failing?"` finishes its single turn
- **THEN** the resume line is printed after the answer

#### Scenario: Piped output stays clean
- **WHEN** stdout is a pipe
- **THEN** the resume line goes to stderr and the piped answer contains no session id

#### Scenario: No session created
- **WHEN** the user runs `imp --list`
- **THEN** no resume line is printed

## ADDED Requirements

### Requirement: Line mode leaves no blank lines

The system SHALL start a tool, info, or error row in line mode on a new line only when the output before it stopped mid-line, and SHALL NOT print an empty line before it otherwise.

#### Scenario: Tool row first
- **WHEN** a one-shot prompt's first output is a tool call
- **THEN** the tool row is the first line printed, with no blank line above it

#### Scenario: After a partial reply
- **WHEN** the reply so far ends without a newline and a tool call follows
- **THEN** the tool row starts on the next line, with no blank line between
