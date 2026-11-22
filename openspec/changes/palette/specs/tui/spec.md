## ADDED Requirements

### Requirement: Ctrl+P opens a palette

The system SHALL open a filterable palette on `Ctrl+P` listing every slash command and the keyboard actions that do something without one, each with its shortcut in the description column, SHALL run the highlighted command on Enter, SHALL replay the highlighted key action on Enter, SHALL close on Esc, and SHALL keep `/` opening the command list alone.

#### Scenario: Open
- **WHEN** the user presses Ctrl+P with an empty draft
- **THEN** a picker titled `palette` lists the commands and the key actions

#### Scenario: Command
- **WHEN** the user filters to `theme` and presses Enter
- **THEN** the theme picker opens, as `/theme` does

#### Scenario: Keyboard action
- **WHEN** the user picks `expand tools` and presses Enter
- **THEN** the latest turn's tool rows expand, as Ctrl+E does

#### Scenario: Filter
- **WHEN** the user types `copy`
- **THEN** the rows narrow to the copy command and any action whose label holds it

#### Scenario: Close
- **WHEN** the user presses Esc
- **THEN** the palette closes and nothing runs

#### Scenario: Sessions
- **WHEN** two live sessions exist and the user presses Ctrl+N
- **THEN** the next session comes into view, and Ctrl+P still opens the palette

#### Scenario: Keys card
- **WHEN** the user opens the keys card
- **THEN** it lists Ctrl+P for the palette and Ctrl+N for the next session
