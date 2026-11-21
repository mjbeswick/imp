## ADDED Requirements

### Requirement: API calls ask by method and credentials

The system SHALL ask in `normal` before every fetch with POST, PUT, PATCH, or DELETE, naming the method, host and path, the body size, and whether credentials are sent, SHALL ask once per host per session before a GET that carries credentials, SHALL ask before every call in `strict`, and SHALL never ask in `yolo`.

#### Scenario: Write
- **WHEN** the mode is `normal` and the model sends two POSTs to the same host
- **THEN** each asks

#### Scenario: Credentialed read
- **WHEN** the mode is `normal` and the model makes two credentialed GETs to `api.github.com`
- **THEN** only the first asks

#### Scenario: Plain read
- **WHEN** the GET carries no credentials
- **THEN** it does not ask, as any fetch
