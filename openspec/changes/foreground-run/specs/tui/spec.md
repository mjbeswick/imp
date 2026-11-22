## MODIFIED Requirements

### Requirement: A shell can be opened from the prompt

The system SHALL suspend the full-screen interface, restore cooked input, and run `$SHELL` in the session directory for `/shell`, SHALL restore the interface and redraw when it exits, SHALL NOT read keys or draw frames while the shell holds the terminal, and SHALL run `$SHELL` directly in line mode.

#### Scenario: Chat shell
- **WHEN** the user runs `/shell` in chat
- **THEN** the normal shell prompt appears and returning to it redraws the chat

#### Scenario: Line mode shell
- **WHEN** the user runs `/shell` in line mode
- **THEN** the shell runs and returns to the prompt

#### Scenario: No shell variable
- **WHEN** `$SHELL` is unset
- **THEN** `/bin/sh` is used

#### Scenario: Session directory
- **WHEN** the user has run `/cd src` and then runs `/shell`
- **THEN** the shell starts in `src`, and exiting it returns to the chat with `shell closed`

#### Scenario: The user's own shell survives
- **WHEN** imp runs from an interactive shell and the user opens `/shell`, types a command, and exits
- **THEN** the command's output shows, imp redraws in full and takes keys, and quitting imp returns to the user's shell

#### Scenario: Keys go to the shell
- **WHEN** the user types twenty commands quickly in `/shell`
- **THEN** all twenty run in the shell and none reaches imp

#### Scenario: No frames over the shell
- **WHEN** `/shell` is open for several seconds
- **THEN** imp draws nothing on the shell's screen
