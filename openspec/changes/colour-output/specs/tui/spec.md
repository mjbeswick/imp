## ADDED Requirements

### Requirement: Shown output keeps its colour

The system SHALL keep colour and text-style sequences in output shown to the user, SHALL drop every other escape sequence and control byte, SHALL cut each row to the screen width by its visible characters with its sequences whole and a reset at the end, SHALL show typed `!` command output this way, and SHALL show plain text when colour is off.

#### Scenario: A picture
- **WHEN** the user runs `!chafa --format symbols photo.jpg`
- **THEN** the picture appears in the transcript in colour

#### Scenario: Escapes that move
- **WHEN** output holds a cursor move, a screen clear, or a title change
- **THEN** none of them reaches the terminal

#### Scenario: Colour off
- **WHEN** the config sets `color=never`
- **THEN** the same output appears without colour

### Requirement: Fenced blocks render through a configured command

The system SHALL render a closed fenced block whose language has a `render.<lang>` command by running that command with the block's text on stdin and `COLUMNS` set to the width, SHALL show its output in place of the block, SHALL show the block as code when the command fails or runs past two seconds, SHALL run each command at most once per width and text, and SHALL keep the block's source in the conversation.

#### Scenario: Mermaid
- **WHEN** the config holds `render.mermaid = mmd` and a reply holds a closed ```` ```mermaid ```` block
- **THEN** the diagram appears in place of the block

#### Scenario: Failure
- **WHEN** the command exits non-zero
- **THEN** the block appears as code

#### Scenario: Streaming
- **WHEN** the block's closing fence has not arrived yet
- **THEN** the block appears as code and the command has not run

#### Scenario: Redraw
- **WHEN** the screen redraws with the same width
- **THEN** the command does not run again
