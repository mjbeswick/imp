## MODIFIED Requirements

### Requirement: Sessions have titles

The system SHALL title a new session from its first user message, cut at a word to at most 40 characters and without a leading filler such as `please` or `can you`, SHALL replace that after the first reply with a summary of at most six words when the message was longer and `title_summary` allows it, SHALL keep a title set with `/rename`, SHALL show the stored title in the title bar and the lists, and SHALL change a title without touching the transcript.

#### Scenario: Automatic title
- **WHEN** a session starts with "fix the flaky rsync test"
- **THEN** the list and the title bar show that as its title

#### Scenario: Long message
- **WHEN** a session starts with "Can you please look at why the rsync test keeps failing on the Pi when the network is slow"
- **THEN** the title bar reads `look at why the rsync test keeps…` at once and a summary such as `Flaky rsync test on slow network` once the first reply ends

#### Scenario: Rename wins
- **WHEN** the user renames the session and a summary arrives afterwards
- **THEN** the renamed title stays

#### Scenario: Rename shows
- **WHEN** the user runs `/rename fix sync`
- **THEN** the title bar reads `fix sync`

#### Scenario: Local provider
- **WHEN** the provider is Ollama and `title_summary` is `auto`
- **THEN** no title request is sent and the cut title stays

#### Scenario: Rename
- **WHEN** the user renames a session
- **THEN** only the sidecar changes and the transcript is untouched
