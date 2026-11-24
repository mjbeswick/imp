# TUI

## ADDED Requirements

### Requirement: /edit opens a terminal editor in the foreground

The system SHALL provide `/edit [target]`, which runs the configured terminal editor in the foreground, SHALL hand the terminal to it, and SHALL restore the chat when the editor exits.

#### Scenario: Edit a file
- **WHEN** the user runs `/edit notes.md`
- **THEN** the terminal editor opens `notes.md` and the chat returns after it exits

#### Scenario: No argument
- **WHEN** the user runs `/edit` with no target
- **THEN** the editor opens the current directory

### Requirement: /open launches a GUI editor without blocking

The system SHALL provide `/open [target]`, which runs the configured GUI editor, or the OS opener when none is set, detached, and SHALL keep the chat running.

#### Scenario: Open in the GUI editor
- **WHEN** the user runs `/open src/main.rs` and `visual` is set
- **THEN** the GUI editor starts with that file and the chat keeps running

#### Scenario: OS opener fallback
- **WHEN** `visual` is unset and no `$VISUAL` is set
- **THEN** `/open` uses `open` on macOS or `xdg-open` on Linux

### Requirement: Editor resolution order

The system SHALL take the editor from the config key first, then from the environment variable, and SHALL split the command into a program and arguments without a shell.

#### Scenario: Config beats environment
- **WHEN** `editor` is set in the config and `$EDITOR` is also set
- **THEN** the config value is used

#### Scenario: Path with spaces
- **WHEN** the target path contains spaces
- **THEN** it is passed as a single argument

### Requirement: Files outside the session directory ask first

The system SHALL ask the user before opening a target outside the session directory, as the file tools do.

#### Scenario: Outside the session
- **WHEN** the target is outside the session directory
- **THEN** the user is asked, and the editor starts only on approval

### Requirement: A missing new file is created on confirmation

The system SHALL create a new file for a target that does not exist only after the user confirms.

#### Scenario: New file
- **WHEN** the user names a path that does not exist
- **THEN** the system asks before creating an empty file there
