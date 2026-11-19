# Spec Delta

## ADDED Requirements

### Requirement: The mouse wheel scrolls the transcript

The system SHALL, when `mouse` is on, ask the terminal for SGR mouse reports while the chat is open, SHALL scroll the transcript three rows per wheel step, SHALL resume following when scrolled to the end, SHALL move a picker's highlight one row per step, SHALL ignore clicks, and SHALL stop mouse reports on exit, on a crash, and while a shell or attached command has the terminal.

#### Scenario: Scroll up
- **WHEN** the transcript is longer than the screen and the user scrolls the wheel up
- **THEN** the view moves up three rows within imp and the terminal's own scrollback is not shown

#### Scenario: Back to the end
- **WHEN** the user scrolls down past the newest row
- **THEN** the view follows new output again

#### Scenario: Picker
- **WHEN** a picker is open and the user scrolls down
- **THEN** the highlight moves down one row

#### Scenario: Exit
- **WHEN** the chat closes or crashes
- **THEN** mouse reporting is off and the terminal handles the mouse again

### Requirement: Mouse capture can be turned off

The system SHALL read a `mouse=on|off` setting, defaulting to `on`, and SHALL send no mouse-reporting request when it is `off`.

#### Scenario: Off
- **WHEN** the config sets `mouse=off`
- **THEN** the chat sends no mouse-reporting request and the wheel behaves as the terminal decides
