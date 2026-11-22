## ADDED Requirements

### Requirement: A run with nothing to do says why

The system SHALL, when a run has no prompt, no interactive terminal, and reads no input, print one line to stderr saying the chat needs a terminal and how to pass a prompt, and SHALL leave the exit code at 0.

#### Scenario: Resumed with no input
- **WHEN** `imp -r` runs with stdin at end of file and no prompt
- **THEN** it prints the resume line, then one line saying the chat needs a terminal, and exits 0

#### Scenario: A pipe with input
- **WHEN** the user runs `echo "fix the parser" | imp`
- **THEN** the prompt runs and no hint is printed

#### Scenario: A prompt argument
- **WHEN** the user runs `imp "fix the parser"` with no terminal
- **THEN** the prompt runs and no hint is printed
