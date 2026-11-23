# providers Specification

## Purpose

Lets one binary speak to several model providers by translating the internal conversation and tool calls into each provider's request and stream format.

## Requirements

### Requirement: A provider profile chooses the endpoint

The system SHALL accept `provider=deepseek|openai|anthropic|gemini|ollama|custom`, SHALL supply a default base URL, model, key variable, and stream dialect for each, and SHALL let `base_url`, `model`, and `api_key` override the profile.

#### Scenario: DeepSeek default
- **WHEN** no provider is set
- **THEN** the session uses the DeepSeek base URL, `deepseek-flash`, and `DEEPSEEK_API_KEY`

#### Scenario: OpenAI
- **WHEN** the user sets `provider=openai` and `OPENAI_API_KEY`
- **THEN** requests go to OpenAI's chat completions endpoint with a Bearer token

#### Scenario: Local model
- **WHEN** the user sets `provider=ollama` and no key
- **THEN** the session starts against `http://localhost:11434` without an auth header

### Requirement: Where the key comes from

The system SHALL resolve a key from `IMP_API_KEY`, then the profile's variable in the environment, then that variable in the keys file in the config directory, then `api_key`, SHALL refuse to start in line mode or a pipeline when a required key is missing by naming the variable it looked for and saying a terminal run can set it, and SHALL offer the chat a provider picker and key entry instead of refusing.

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

### Requirement: OpenAI-compatible providers share one adapter

The system SHALL send `chat/completions` requests with `messages`, `tools`, and a stream flag, SHALL parse `choices[].delta` content, reasoning, and tool calls, SHALL accept usage fields when present and tolerate their absence, and SHALL use `max_completion_tokens` instead of `max_tokens` for models that require it.

#### Scenario: OpenAI stream
- **WHEN** a streamed reply carries content deltas
- **THEN** the text appears in order and the finish reason ends the turn

#### Scenario: No cache fields
- **WHEN** the provider reports usage without cache counts
- **THEN** the session records zero cache tokens and the turn continues

#### Scenario: Reasoning effort
- **WHEN** thinking is on for a model that supports `reasoning_effort`
- **THEN** that field is sent and no DeepSeek `thinking` object is included

### Requirement: Anthropic has its own adapter

The system SHALL send `POST /v1/messages` with `x-api-key` and `anthropic-version` headers, SHALL move system messages into the top-level `system` field, SHALL translate tools to `input_schema`, SHALL send tool results as `tool_result` blocks, and SHALL parse `content_block_delta`, `message_delta`, and `message_stop` events into text, reasoning, and tool calls.

#### Scenario: Anthropic text
- **WHEN** the stream emits `text_delta` blocks
- **THEN** the text arrives in order and the message stops cleanly

#### Scenario: Anthropic tools
- **WHEN** the model calls `bash`
- **THEN** a `tool_use` block becomes a tool call and the result returns as a `tool_result`

#### Scenario: Thinking block
- **WHEN** the stream emits a `thinking_delta`
- **THEN** it becomes reasoning content, not answer text

### Requirement: Gemini has its own adapter

The system SHALL call `streamGenerateContent` with `alt=sse` and `x-goog-api-key`, SHALL send `systemInstruction` and `contents`, SHALL translate tools to `functionDeclarations`, SHALL parse text and `functionCall` parts, and SHALL map `usageMetadata` to prompt, output, and cached token counts.

#### Scenario: Gemini text
- **WHEN** a candidate part carries text
- **THEN** the text streams into the reply

#### Scenario: Gemini function call
- **WHEN** a part carries a `functionCall`
- **THEN** it becomes an internal tool call with the same name and arguments

#### Scenario: Usage
- **WHEN** `usageMetadata` reports token counts
- **THEN** the status line's token and cache figures reflect them

### Requirement: Capabilities gate optional fields and tools

The system SHALL omit fields a provider does not accept, SHALL expose `search` only when the provider implements the search and fetch endpoints, and SHALL keep the prompt free of the search line when the tool is absent.

#### Scenario: No search on OpenAI
- **WHEN** the active provider is OpenAI
- **THEN** the tool schema has no `search` and the prompt has no search line

#### Scenario: DeepSeek search
- **WHEN** the active provider is DeepSeek
- **THEN** `search` and `fetch` are available as today

#### Scenario: Thinking unsupported
- **WHEN** a provider has no thinking field
- **THEN** no thinking or reasoning field is sent even if `thinking=true`

### Requirement: Cost is reported only where known

The system SHALL show a price only for models with a known price, SHALL keep showing tokens for every provider, and SHALL NOT guess a price for an unknown model.

#### Scenario: Unknown price
- **WHEN** the provider is OpenAI and no price entry exists
- **THEN** the status line shows tokens and no cost

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

### Requirement: Anthropic requests carry no sampling parameters

The system SHALL send no `temperature`, `top_p`, or `top_k` in requests to the Anthropic provider, whatever the model, and SHALL list current Claude models in the built-in catalog, with `claude-opus-5-5` as the provider default.

#### Scenario: Current model
- **WHEN** the session model is `claude-opus-5-5` and a turn starts
- **THEN** the request body has no `temperature`

#### Scenario: Catalog
- **WHEN** the models endpoint is unreachable and the user opens `/model` on Anthropic
- **THEN** the picker lists `claude-opus-5-5`, `claude-sonnet-5-5`, and `claude-haiku-5-5`

### Requirement: Effort is a separate dial

The system SHALL keep an effort setting of `low`, `medium`, or `high`, unset by default, separate from the thinking switch, SHALL send it as `reasoning_effort` to OpenAI-compatible providers, sending `low` when unset, SHALL send it as `output_config.effort` to Anthropic when set, and SHALL note once per session when the provider has no such dial.

#### Scenario: Anthropic
- **WHEN** effort is `high` and the provider is Anthropic
- **THEN** the request body has `output_config.effort` set to `high`

#### Scenario: Unset
- **WHEN** effort is unset and the provider is DeepSeek with thinking on
- **THEN** the request sends `reasoning_effort: low`, as before

#### Scenario: No dial
- **WHEN** the provider is Ollama and the user runs `/effort high`
- **THEN** one line says the provider has no effort setting
