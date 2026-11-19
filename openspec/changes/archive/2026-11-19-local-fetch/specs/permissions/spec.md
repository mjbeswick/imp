# Spec Delta

## ADDED Requirements

### Requirement: Fetches ask once per host

The system SHALL ask before the first `fetch` from a host in a session in `normal`, SHALL not ask again for that host in that session, SHALL ask before every fetch in `strict`, and SHALL never ask in `yolo`. A refusal SHALL be a tool result and SHALL NOT approve the host.

#### Scenario: First fetch
- **WHEN** the mode is `normal` and the model fetches from `wttr.in` for the first time
- **THEN** the prompt reads `fetch from wttr.in for this session? y/N`

#### Scenario: Same host again
- **WHEN** the user approved `wttr.in` and the model fetches another `wttr.in` URL
- **THEN** no prompt appears

#### Scenario: Another host
- **WHEN** the model then fetches from `feeds.bbci.co.uk`
- **THEN** the prompt appears for that host

#### Scenario: Strict
- **WHEN** the mode is `strict`
- **THEN** every fetch asks

#### Scenario: Yolo
- **WHEN** the mode is `yolo`
- **THEN** no fetch asks
