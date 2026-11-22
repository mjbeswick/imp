## MODIFIED Requirements

### Requirement: Every rendered line fits the terminal

The system SHALL clamp every drawn row to the terminal width, SHALL disable the terminal's automatic wrapping while drawing and restore it after, SHALL erase the rest of each row after writing it and never blank a row before writing it, SHALL wrap each frame in synchronised output, SHALL write only the rows that differ from the previous frame, SHALL clear everything below the status line, and SHALL begin each row at column one.

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

#### Scenario: No blank rows between frames
- **WHEN** a confirmation counts down from 28s to 27s
- **THEN** each row of the new frame is written before the rest of that row is erased, and no full-row erase precedes a row's text

#### Scenario: Synchronised frame
- **WHEN** a frame is drawn
- **THEN** it starts with `ESC[?2026h` and ends with `ESC[?2026l`

#### Scenario: Only changed rows
- **WHEN** a confirmation counts down from 28s to 27s and nothing else changed
- **THEN** only the confirmation row is written to the terminal

#### Scenario: Full repaint when needed
- **WHEN** the terminal is resized, or a picker opens or closes
- **THEN** every row is written
