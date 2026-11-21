## ADDED Requirements

### Requirement: Downloads into scratch are fetches

The system SHALL treat a single `curl -o` or `wget -O` download of one URL whose target lies under `$IMP_TMP`, with no chaining, pipes, redirects, other substitution, or `..`, as a fetch of that URL, and SHALL treat `chafa` as a safe-listed command.

#### Scenario: Image download
- **WHEN** the agent runs `curl -fsSL -o "$IMP_TMP/a.jpg" https://example.com/a.jpg` in normal mode
- **THEN** it runs without a prompt

#### Scenario: Elsewhere still asks
- **WHEN** the agent runs `curl -o /tmp/a.jpg https://example.com/a.jpg`
- **THEN** imp asks first

#### Scenario: Escaping scratch asks
- **WHEN** the target is `$IMP_TMP/../x`
- **THEN** imp asks first

#### Scenario: Strict asks
- **WHEN** the mode is strict
- **THEN** a scratch download asks first
