# Proposal

## Why

Imp speaks exactly one dialect: DeepSeek's OpenAI-compatible stream plus its
custom search endpoints. `base_url` exists, but every request still sends
DeepSeek's `thinking` object, auth is Bearer-only, and the search tool calls
`/v1/tools/search` on the provider host. Pointing it at OpenAI, Anthropic,
Gemini, or a local model fails at the first request.

## What Changes

- Add a `provider` setting with profiles for DeepSeek, OpenAI-compatible hosts,
  Anthropic, Gemini, Ollama, and a custom base URL.
- Build requests and parse streams per provider: OpenAI `chat/completions`,
  Anthropic `messages`, Gemini `streamGenerateContent`.
- Resolve keys from the provider's environment variable, then `IMP_API_KEY`,
  then the config; report the right name when missing.
- Gate capabilities per provider: thinking fields, token-limit key, cache
  usage, and the search tool. Providers without search endpoints do not get the
  tool or its prompt line.
- Keep cost reporting DeepSeek-only; other providers show tokens and no price.

## Capabilities

### New Capabilities

- `providers`: provider selection, defaults, key resolution, the three wire
  formats, streaming adapters, and capability gating.

### Modified Capabilities

- `tools`: the search tool is present only when the active provider supports
  the endpoints.

## Impact

- `src/providers/`: new modules for the shared trait, OpenAI-compatible,
  Anthropic, and Gemini adapters.
- `src/api.rs`: request building and stream parsing move behind the adapter.
- `src/config.rs`: provider profiles, defaults, and key resolution.
- `src/agent.rs`: capability-driven tool schema and search dispatch.
- `src/prompt.rs`: conditional search line.
- `src/help.rs`, `README.md`, and the specs.
