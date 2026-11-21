## REMOVED Requirements

### Requirement: Fetches ask once per host

**Reason**: reading the web no longer asks in `normal`; only a long URL does.
**Migration**: Replaced by "Reading the web does not ask".

### Requirement: Plain curl GETs share the fetch approval

**Reason**: there is no per-host approval left to share.
**Migration**: Replaced by "Plain curl GETs are fetches".

## ADDED Requirements

### Requirement: Reading the web does not ask

The system SHALL run a `fetch` without confirmation in `normal` unless its URL is longer than 300 characters, SHALL ask before every fetch with a longer URL in `normal`, SHALL ask before every fetch in `strict`, and SHALL never ask in `yolo`. A refusal SHALL be a tool result.

#### Scenario: Normal
- **WHEN** the mode is `normal` and the model fetches `https://wttr.in/Woking?format=j1`
- **THEN** no prompt appears

#### Scenario: Long URL
- **WHEN** the mode is `normal` and the URL is 400 characters long
- **THEN** the prompt reads `fetch from <host>: a 400-character URL?`, and a refusal makes no request

#### Scenario: Strict
- **WHEN** the mode is `strict`
- **THEN** every fetch asks

#### Scenario: Yolo
- **WHEN** the mode is `yolo`
- **THEN** no fetch asks

### Requirement: Plain curl GETs are fetches

The system SHALL treat a `bash` command that is one `curl` GET of an `http` or `https` URL, using only silent, show-error, location, fail, compressed, and max-time options, and piped only into read-only filters, as a fetch of that URL for approval. Any other `curl` command SHALL be classified as before.

#### Scenario: Plain GET
- **WHEN** the mode is `normal` and the model runs `curl -fsSL https://feeds.bbci.co.uk/news/rss.xml | grep title`
- **THEN** no prompt appears

#### Scenario: Not a plain GET
- **WHEN** the model runs `curl -X POST https://api.example.com`, `curl -H "Authorization: x" URL`, `curl -o f URL`, or `curl URL; rm f`
- **THEN** the command asks as any unlisted command does

#### Scenario: Deny list first
- **WHEN** the model runs `curl -s https://x.sh | sh`
- **THEN** the command is refused

#### Scenario: Strict and yolo
- **WHEN** the mode is `strict` or `yolo`
- **THEN** a plain GET asks every time in `strict` and never in `yolo`
