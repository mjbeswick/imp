# Spec Delta

## Purpose

Defines the terminal interface: what the user sees while a turn runs, which keys exist, what the status line reports, and how the terminal is left after any exit.

## MODIFIED Requirements

### Requirement: Every rendered line fits the terminal

The system SHALL clamp every drawn row to the terminal width, SHALL disable the terminal's automatic wrapping while drawing and restore it after, SHALL erase each row before writing it, SHALL clear everything below the status line, and SHALL begin each row at column one.

#### Scenario: Narrow terminal
- **WHEN** the terminal is 40 columns and the status line is longer
- **THEN** the status is truncated with an ellipsis and no row wraps onto the next

#### Scenario: Shrinking window
- **WHEN** the window shrinks between frames
- **THEN** the next frame has no leftover text from the wider one

#### Scenario: No debris
- **WHEN** a menu closes
- **THEN** the rows it used are erased and the chat pane grows back

#### Scenario: Raw terminal mode
- **WHEN** the TUI renders a frame while terminal output processing is disabled
- **THEN** each row boundary emits carriage return and line feed before the next row

#### Scenario: Empty rows before the input
- **WHEN** a short conversation leaves blank rows above the input
- **THEN** the input prompt begins at column one
