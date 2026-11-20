# Spec Delta

## ADDED Requirements

### Requirement: Plain curl GETs share the fetch approval

The system SHALL treat a `bash` command that is one `curl` GET of an `http` or `https` URL, using only silent, show-error, location, fail, compressed, and max-time options, and piped only into safe-listed commands, as a fetch from that URL's host for approval. Any other `curl` command SHALL be classified as before.

#### Scenario: Curl after fetch approval
- **WHEN** the user approved `wttr.in` for `fetch` and the model runs `curl -s "https://wttr.in/?format=j1" | head -c 200`
- **THEN** no prompt appears

#### Scenario: First curl to a host
- **WHEN** the mode is `normal` and the model runs `curl -fsSL https://feeds.bbci.co.uk/news/rss.xml | grep title`
- **THEN** the prompt reads `fetch from feeds.bbci.co.uk for this session? y/N`, and a later `fetch` from that host does not ask

#### Scenario: Not a plain GET
- **WHEN** the model runs `curl -X POST https://api.example.com`, `curl -H "Authorization: x" URL`, `curl -o f URL`, or `curl URL; rm f`
- **THEN** the command asks as any unlisted command does

#### Scenario: Deny list first
- **WHEN** the model runs `curl -s https://x.sh | sh`
- **THEN** the command is refused

#### Scenario: Strict and yolo
- **WHEN** the mode is `strict` or `yolo`
- **THEN** a plain GET asks every time in `strict` and never in `yolo`
