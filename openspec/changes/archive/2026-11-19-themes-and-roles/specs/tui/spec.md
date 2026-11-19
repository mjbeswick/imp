# Spec Delta

## MODIFIED Requirements

### Requirement: The interface uses colour with meaning

The system SHALL colour by role rather than by text, SHALL paint inline code bold in the code colour and keep fenced blocks and quotes muted, SHALL detect truecolor, 256-colour, and no-colour terminals, and SHALL honour `NO_COLOR` and a `color=auto|always|never` setting.

#### Scenario: Truecolor terminal
- **WHEN** `COLORTERM` advertises truecolor
- **THEN** roles render as 24-bit RGB

#### Scenario: Plain terminal
- **WHEN** the terminal supports 256 colours only
- **THEN** the same roles render from a 256-colour palette that keeps the same contrast relationships

#### Scenario: No colour
- **WHEN** `NO_COLOR` is set or the setting is `never`
- **THEN** no escape sequences are emitted and the layout is unchanged

#### Scenario: Error stands out
- **WHEN** a tool exits non-zero or a request fails
- **THEN** that line renders in the error role and remains readable without colour

#### Scenario: Inline code is not dim
- **WHEN** a reply contains `` `bash` `` inside a sentence
- **THEN** that span is bold in the code colour, not the muted grey

#### Scenario: Fenced code stays dim
- **WHEN** a reply contains a fenced block
- **THEN** its lines are dim and not bold

### Requirement: Themes are selectable

The system SHALL provide `default`, `default-light`, `solarized-dark`, `solarized-light`, `gruvbox-dark`, `gruvbox-light`, `catppuccin-mocha`, `catppuccin-latte`, `tokyo-night`, `nord`, and `mono`, SHALL apply the active theme to both front-ends, SHALL switch with `/theme <name>`, SHALL reject an unknown name and list the valid ones, and SHALL save a switch and load it at startup. `mono` SHALL force colour off.

#### Scenario: List
- **WHEN** the user runs `/theme`
- **THEN** the eleven names appear with the active one marked

#### Scenario: Switch
- **WHEN** the user runs `/theme gruvbox-dark`
- **THEN** the next frame and the next line-mode rows use that palette

#### Scenario: Mono
- **WHEN** the user runs `/theme mono`
- **THEN** no colour is emitted even in a colour terminal

#### Scenario: Unknown
- **WHEN** the user runs `/theme neon`
- **THEN** the transcript refuses it and lists the valid names

#### Scenario: Persistence
- **WHEN** the user switches to `solarized-light` and restarts
- **THEN** the session starts in `solarized-light`

#### Scenario: Bad saved value
- **WHEN** the state file names a theme that does not exist
- **THEN** the session starts in `default` or `default-light` by the background guess

## ADDED Requirements

### Requirement: Eleven roles cover the screen

The system SHALL give every theme eleven roles, `text`, `user`, `muted`, `border`, `accent`, `tool`, `success`, `error`, `code`, `link`, and `select`, SHALL paint rules and the picker frame in `border`, `✓` marks and added lines in `success`, `✗` marks and removed lines in `error`, links in `link`, and the highlighted picker row on the `select` background.

#### Scenario: Picker highlight
- **WHEN** a picker is open in a colour terminal
- **THEN** the highlighted row has the `select` background across the inner width

#### Scenario: Marks
- **WHEN** a tool call succeeds
- **THEN** its `✓` is bold in the success colour and the summary is in the tool colour

### Requirement: The default follows the terminal background

The system SHALL, when no theme is saved, choose `default-light` for a light terminal background and `default` otherwise, SHALL read `COLORFGBG` first, SHALL then ask the chat's terminal with OSC 11 and wait at most 100 ms, and SHALL treat no answer as dark.

#### Scenario: Light terminal
- **WHEN** no theme is saved and the terminal reports a white background
- **THEN** the session starts in `default-light`

#### Scenario: Silent terminal
- **WHEN** no theme is saved and the terminal does not answer within 100 ms
- **THEN** the session starts in `default` without delay beyond that limit

#### Scenario: Saved choice wins
- **WHEN** a theme is saved
- **THEN** no query is sent and the saved theme is used
