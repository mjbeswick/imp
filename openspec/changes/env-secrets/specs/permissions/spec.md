## ADDED Requirements

### Requirement: Secrets stay out of the agent's commands

The system SHALL remove variables whose names look like credentials from the environment of commands and jobs the agent runs, unless named in `pass_env`, SHALL replace each such value with `[redacted NAME]` in tool results and typed command output, SHALL ask before running an agent command that names a removed variable in `normal` and `strict`, and SHALL leave commands the user types with their full environment.

#### Scenario: Stripped
- **WHEN** `GITHUB_TOKEN` is set and the agent runs `printenv GITHUB_TOKEN`
- **THEN** the command sees no such variable

#### Scenario: Passed through
- **WHEN** the config holds `pass_env = GITHUB_TOKEN`
- **THEN** the agent's commands see `GITHUB_TOKEN`

#### Scenario: Redacted
- **WHEN** a command prints a file that holds the value of `DEEPSEEK_API_KEY`
- **THEN** the result reads `[redacted DEEPSEEK_API_KEY]` in place of the value

#### Scenario: Named
- **WHEN** the agent runs `echo $DEEPSEEK_API_KEY` in `normal`
- **THEN** a confirmation is required

#### Scenario: Typed
- **WHEN** the user types `!gh auth status` with `GH_TOKEN` set
- **THEN** the command sees `GH_TOKEN`
