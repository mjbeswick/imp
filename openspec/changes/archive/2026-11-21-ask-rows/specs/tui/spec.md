## MODIFIED Requirements

### Requirement: Pending prompts stand out

The system SHALL paint an open confirmation, question, or input prompt in the accent role behind a `▌` prefix, SHALL paint the rule above it in the accent role while it is open, SHALL draw a text prompt's caret the same way as the composer's, and SHALL keep a confirmation or text prompt to one row, while an `ask` question takes a row for the question, one per choice, and one for the keys, each wrapped at the screen width.

#### Scenario: Confirmation
- **WHEN** a confirmation opens in a colour terminal
- **THEN** its row starts with `▌` in the accent colour and the rule above it is in the accent colour

#### Scenario: No colour
- **WHEN** colour is off and a confirmation opens
- **THEN** the row starts with `▌` and no escape sequence is emitted for colour

#### Scenario: Text prompt caret
- **WHEN** `/rename` asks for a title
- **THEN** the caret is the composer's block caret, not an underscore

#### Scenario: Long choices
- **WHEN** an ask question's choices together are wider than the terminal
- **THEN** each choice starts its own row with its number key, wraps instead of being cut, and the question stays above them
