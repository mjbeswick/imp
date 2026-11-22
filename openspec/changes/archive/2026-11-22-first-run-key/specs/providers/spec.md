## MODIFIED Requirements

### Requirement: A provider profile chooses the endpoint

The system SHALL accept `provider=deepseek|openai|anthropic|gemini|ollama|custom`, SHALL supply a default base URL, model, key variable, and stream dialect for each, SHALL let `base_url`, `model`, and `api_key` override the profile, SHALL resolve a key from `IMP_API_KEY`, then the profile's variable in the environment, then that variable in the keys file in the config directory, then `api_key`, SHALL refuse to start in line mode or a pipeline when a required key is missing by naming the variable it looked for and saying a terminal run can set it, and SHALL offer the chat a provider picker and key entry instead of refusing.

#### Scenario: DeepSeek default
- **WHEN** no provider is set
- **THEN** the session uses the DeepSeek base URL, `deepseek-flash`, and `DEEPSEEK_API_KEY`

#### Scenario: OpenAI
- **WHEN** the user sets `provider=openai` and `OPENAI_API_KEY`
- **THEN** requests go to OpenAI's chat completions endpoint with a Bearer token

#### Scenario: Local model
- **WHEN** the user sets `provider=ollama` and no key
- **THEN** the session starts against `http://localhost:11434` without an auth header

#### Scenario: Missing key
- **WHEN** `provider=anthropic`, no key is stored, and the run is a pipeline
- **THEN** imp exits with an error naming `ANTHROPIC_API_KEY` and saying a terminal run can set it

#### Scenario: Environment wins
- **WHEN** `DEEPSEEK_API_KEY` is exported and the keys file holds another value
- **THEN** the exported key is used

#### Scenario: Stored key
- **WHEN** no variable is set and the keys file holds `DEEPSEEK_API_KEY=sk-...`
- **THEN** the session uses that key

#### Scenario: Custom host
- **WHEN** `provider=custom` and `base_url` is set
- **THEN** the profile's stream dialect and key variable apply, and a stored key is used when the variable is unset

## ADDED Requirements

### Requirement: First run picks a provider

The system SHALL, in the chat with no key for the active provider, open the provider picker listing the built-in profiles and the named providers with the active one highlighted, SHALL apply a profile that needs no key at once, SHALL otherwise open a masked key row naming the profile's variable, and SHALL quit on Esc.

#### Scenario: Picker
- **WHEN** the chat starts with `provider=deepseek` and no key
- **THEN** the picker lists the built-in profiles and the named providers, with DeepSeek highlighted

#### Scenario: No key needed
- **WHEN** the user picks Ollama
- **THEN** the chat starts at once with no key row

#### Scenario: Escape
- **WHEN** the user presses Esc on the key row or the picker
- **THEN** the chat closes without writing a key

### Requirement: The key entry checks and saves

The system SHALL mask the key row with one star per character, SHALL append typed characters and remove them with Backspace, SHALL on Enter check the key with one provider request, SHALL save it and apply the provider on success, SHALL keep the row open and show the server's answer on a 4xx, SHALL save it with a warning when the check cannot reach the server, and SHALL write the keys file read and writable by its owner only.

#### Scenario: Typed key
- **WHEN** the user picks DeepSeek and types a working key
- **THEN** the row shows stars, the key is saved, the chat starts on `deepseek-flash`, and no error is shown

#### Scenario: Wrong key
- **WHEN** the provider answers `HTTP 401`
- **THEN** the row stays open with the error and the file is unchanged

#### Scenario: Offline
- **WHEN** the check cannot reach the server
- **THEN** the key is saved, the chat starts, and a warning says the check did not run

#### Scenario: File mode
- **WHEN** the key is saved
- **THEN** the keys file is readable and writable by the owner only
