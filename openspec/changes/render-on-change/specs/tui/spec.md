# Spec Delta

## ADDED Requirements

### Requirement: The interface redraws only when something changes

The system SHALL render a frame only when a key was handled, an event arrived from the session, a resize was noticed, or a visible state changed, SHALL NOT repaint an idle screen on a timer, and SHALL draw the streaming spinner at no more than ten frames a second.

#### Scenario: Idle screen
- **WHEN** the interface sits at an empty prompt with no keys and no events
- **THEN** it writes nothing to the terminal until something changes

#### Scenario: Spinner
- **WHEN** a turn is streaming and the spinner advances
- **THEN** frames are at least 100 ms apart

#### Scenario: Keystroke still repaints
- **WHEN** the user types a character
- **THEN** the frame is redrawn immediately
