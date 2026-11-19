# providers Specification

## Purpose

Lets one binary speak to several model providers by translating the internal conversation and tool calls into each provider's request and stream format.

## Requirements

### Requirement: A provider profile chooses the endpoint

The system SHALL accept `provider=deepseek|openai|anthropic|gemini|ollama|custom`, SHALL supply a default base URL, model, key variable, and stream dialect for each, SHALL let `base_url`, `model`, and `api_key` override the profile, and SHALL refuse to start when a required key is missing by naming the variable it looked for.

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
- **WHEN** `provider=anthropic` and no key is set
- **THEN** imp exits with an error naming `ANTHROPIC_API_KEY`

#### Scenario: Custom host
- **WHEN** `provider=custom` and `base_url` is set
- **THEN** the OpenAI-compatible chat path is used at that host

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
