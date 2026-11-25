## ADDED Requirements

### Requirement: Command palette

The system SHALL open a command palette with Ctrl+P on an empty composer, lists every command from the `/help` table with its description, SHALL filter that list as the user types, SHALL run the highlighted command with Enter, SHALL close the palette with Esc and leave the composer unchanged, and SHALL keep the slash commands working as before.

#### Scenario: Open the palette
- **WHEN** the user presses Ctrl+P with no other picker open and the composer empty
- **THEN** the palette opens with every command listed, each with its description, and the filter empty

#### Scenario: Filter the palette
- **WHEN** the user types `qu` in the palette
- **THEN** only commands whose names contain `qu` remain, such as `/quit`, and the highlight moves to the first

#### Scenario: Run a command from the palette
- **WHEN** the user filters to `/quit` and presses Enter
- **THEN** the session quits, as it does for `/quit` typed in the composer

#### Scenario: Close the palette
- **WHEN** the user presses Esc in the palette
- **THEN** the palette closes and the composer text is unchanged

#### Scenario: Slash commands unchanged
- **WHEN** the user types `/quit` in the composer
- **THEN** the session quits, as before this change
