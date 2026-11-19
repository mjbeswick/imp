# Spec Delta

## MODIFIED Requirements

### Requirement: The tool set is small and shell-first

The system SHALL expose `bash`, `read`, `edit`, `write`, `job`, `ask`, and `fetch` as its core tool set, SHALL expose `search` only when the active provider implements the search endpoint, SHALL NOT expose per-CLI tools such as `gh`, `docker`, `kubectl`, `git`, glob, grep, list, patch, or todo, and SHALL teach specific command-line tools through skills instead.

#### Scenario: Default prompt
- **WHEN** a session starts with memory and skills configured
- **THEN** the request carries the seven core tools plus `memory` and `skill_load`, and no per-CLI tool

#### Scenario: A CLI is not installed as a tool
- **WHEN** the model needs `gh`
- **THEN** it loads the `gh` skill if one is installed and runs `gh` through `bash`, rather than calling a `gh` tool

#### Scenario: File discovery without a glob tool
- **WHEN** the model needs to find files by pattern
- **THEN** it uses `bash` and the task completes without a dedicated tool

#### Scenario: No search capability
- **WHEN** the provider has no search endpoint
- **THEN** the tool list omits `search`, keeps `fetch`, and the prompt has no search line

### Requirement: Fetch returns page text as markdown

The system SHALL provide a local `fetch` tool for every provider that makes one GET request over `http` or `https` with a 15 second limit, SHALL return the status line and the body, SHALL return JSON, XML, and text unchanged and HTML as its title and readable text, and SHALL apply the 4 KB cap and spill rule.

#### Scenario: Reading an article
- **WHEN** the model fetches an HTML page
- **THEN** it receives the status line, the page title, and the page text without scripts or tags

#### Scenario: A markdown file
- **WHEN** the model fetches a `.md` file with headings
- **THEN** the result is the file's text as served, beginning with its first line

#### Scenario: A JSON API
- **WHEN** the model fetches `https://api.open-meteo.com/v1/forecast?...`
- **THEN** it receives `200 application/json` and the JSON body unchanged

#### Scenario: Local network
- **WHEN** the model fetches `http://192.168.0.61:11080/`
- **THEN** the request is made from this machine and the device answers

#### Scenario: Other schemes
- **WHEN** the model fetches `file:///etc/passwd`
- **THEN** it is refused and no request is made

### Requirement: Search returns summaries, not pages

The system SHALL provide a `search` tool, on providers that implement it, that queries the web and returns at most five results containing title, URL, summary, date, and a short snippet, and SHALL NOT fetch or return page bodies.

#### Scenario: Web question
- **WHEN** the model searches for a current fact
- **THEN** it receives summaries with URLs in one round trip and no page content

#### Scenario: No results
- **WHEN** a search returns an empty result list
- **THEN** the result says so plainly and is reported as unbilled
