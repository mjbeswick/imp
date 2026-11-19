# Spec Delta

## ADDED Requirements

### Requirement: The theme picker previews

The system SHALL repaint the screen in the highlighted theme while the theme picker is open without saving it, SHALL apply and save the highlighted theme on Enter, and SHALL restore the theme active when the picker opened on Esc, Ctrl+C, or any other close.

#### Scenario: Move
- **WHEN** the user opens `/theme` and presses Down
- **THEN** the screen repaints in the next theme and the saved theme is unchanged

#### Scenario: Cancel
- **WHEN** the user then presses Esc
- **THEN** the screen returns to the theme active before the picker opened

#### Scenario: Choose
- **WHEN** the user presses Enter on `nord`
- **THEN** `nord` stays active after the picker closes and is saved
