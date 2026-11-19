# Spec Delta

## ADDED Requirements

### Requirement: Yolo shows in colour

The system SHALL paint the composer prompt glyph and the rule above the composer in the error colour while the live mode is `yolo`, SHALL paint the line-mode prompt the same way, SHALL leave the status line text unchanged, and SHALL emit nothing extra when colour is off.

#### Scenario: Yolo on
- **WHEN** the user runs `/yolo` in a colour terminal
- **THEN** the next frame paints `> ` and the rule in the error colour

#### Scenario: Back to normal
- **WHEN** the user runs `/yolo` again
- **THEN** the glyph and rule return to their usual colours

#### Scenario: No colour
- **WHEN** colour is off and the mode is `yolo`
- **THEN** the frame is identical to the `normal` frame
