## ADDED Requirements

### Requirement: The agent can show output to the user

The system SHALL accept `show` on `bash`, SHALL show the command's output to the user, up to 200 lines, with its colour kept, SHALL give the model the output as plain text, and SHALL give the model only a placeholder naming the size when more than a third of the visible characters are block or braille characters.

#### Scenario: A chart
- **WHEN** the model runs a command that prints a coloured chart with `show`
- **THEN** the user sees the chart in colour and the model receives its text

#### Scenario: A picture
- **WHEN** the model runs `chafa --format symbols --size 60x24 photo.jpg` with `show`
- **THEN** the user sees the picture and the model receives `[shown to the user: a 60×24 picture]`
