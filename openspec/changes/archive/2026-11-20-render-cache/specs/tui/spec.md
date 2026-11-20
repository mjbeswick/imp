## ADDED Requirements

### Requirement: Redraws reuse rendered rows

The system SHALL keep each transcript entry's rendered rows between frames, SHALL render an entry again only when its text, its role, the terminal width, the markdown setting, or the blank row before it changes, and SHALL draw the same screen as a full render.

#### Scenario: Streaming
- **WHEN** a reply streams at the end of a long transcript
- **THEN** only the streaming entry is rendered again on each frame

#### Scenario: Resize
- **WHEN** the terminal width changes
- **THEN** every entry is rendered again at the new width

#### Scenario: Same screen
- **WHEN** a frame is drawn from cached rows
- **THEN** it matches a frame drawn with an empty cache
