# Spec Delta

## MODIFIED Requirements

### Requirement: The tool set is small and shell-first

The system SHALL expose `bash`, `read`, `edit`, `write`, `job`, and `ask` as its core tool set, SHALL expose `search` only when the active provider implements the search and fetch endpoints, SHALL NOT expose per-CLI tools such as `gh`, `docker`, `kubectl`, `git`, glob, grep, list, patch, or todo, and SHALL teach specific command-line tools through skills instead.

#### Scenario: Default prompt
- **WHEN** a session starts with memory and skills configured
- **THEN** the request carries the provider's tools plus `memory` and `skill_load`, and no per-CLI tool

#### Scenario: A CLI is not installed as a tool
- **WHEN** the model needs `gh`
- **THEN** it loads the `gh` skill if one is installed and runs `gh` through `bash`, rather than calling a `gh` tool

#### Scenario: File discovery without a glob tool
- **WHEN** the model needs to find files by pattern
- **THEN** it uses `bash` and the task completes without a dedicated tool

#### Scenario: No search capability
- **WHEN** the provider has no search endpoints
- **THEN** the tool list omits `search` and the prompt has no search line
