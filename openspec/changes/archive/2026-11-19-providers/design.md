# Design

## Approaches

1. **A provider enum plus adapters behind one internal event stream.** Chosen:
   the session loop keeps using `Event`, and each adapter owns its request body,
   auth, URL, and chunk translation.
2. **One generic OpenAI-compatible client and proxy shims.** Anthropic and
   Gemini would need a local translation server; far more moving parts.
3. **Separate binaries per provider.** Duplicates the agent loop and tools.

## Decisions

- `Provider` is `deepseek`, `openai`, `anthropic`, `gemini`, `ollama`, or
  `custom`, with a `Caps` record: thinking style, token-limit key, cache usage,
  search endpoints, and max-token field.
- Profiles supply base URL, model, and key variable:
  - DeepSeek: `https://api.deepseek.com`, `deepseek-flash`, `DEEPSEEK_API_KEY`
  - OpenAI: `https://api.openai.com/v1`, `gpt-4.1-mini`, `OPENAI_API_KEY`
  - Anthropic: `https://api.anthropic.com`, a current Sonnet model,
    `ANTHROPIC_API_KEY`
  - Gemini: `https://generativelanguage.googleapis.com`, a current Pro model,
    `GEMINI_API_KEY`
  - Ollama: `http://localhost:11434/v1`, a small local default, no key
  - Custom: user base URL and model, OpenAI-compatible dialect
- Key order: config `api_key`, then `IMP_API_KEY`, then the provider variable.
  Ollama requires none.
- `src/providers/` holds `mod.rs`, `openai.rs`, `anthropic.rs`, `gemini.rs`.
  The adapter trait returns the endpoint, headers, request body, and a parser
  that turns one SSE payload into `Event`s.
- OpenAI keeps today's body and parser, with the DeepSeek `thinking` object
  only under the DeepSeek capabilities and `reasoning_effort` under the
  reasoning-capable profiles.
- Anthropic moves system messages to top-level `system`, emits user and
  assistant content blocks, maps tool results to `tool_result`, and parses
  `text_delta`, `thinking_delta`, and `input_json_delta`. Stop reasons map to
  the internal finish values.
- Gemini sends `systemInstruction` and `contents`, maps tool calls to
  `functionCall` and results to `functionResponse`, and reads
  `usageMetadata` into prompt, output, and cached counts.
- `search` is added to the schema and prompt only when the provider has the
  endpoints. DeepSeek keeps them; the rest omit both.
- Cost stays in the DeepSeek price table. Other models return `None` and the
  status line shows tokens only.
- Images, when `image-paste` lands, need per-provider parts; this change leaves
  a `content_parts` hook in each adapter so that change does not rewrite them.
- Tests use the existing `Fake` transport with one recorded stream per
  provider, plus body snapshots, key precedence, capability filtering, and an
  unknown-price status line.
