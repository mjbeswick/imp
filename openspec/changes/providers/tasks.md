# Tasks

- [x] 1.1 Add provider profiles and key resolution. Parse `provider`, default base/model/key variable, `IMP_PROVIDER`, and `--provider`; resolve config, `IMP_API_KEY`, then the provider variable; name the missing variable on failure. Verify: each profile yields its endpoint and key, Ollama needs none, and a missing Anthropic key names `ANTHROPIC_API_KEY`.
- [x] 1.2 Introduce the adapter trait and move the existing OpenAI/DeepSeek path behind it. Endpoint, headers, body, and chunk parser become methods; DeepSeek-only fields come from capabilities. Verify: all existing stream fixtures still pass and the DeepSeek body is unchanged.
- [x] 1.3 Add the Anthropic adapter. Translate system messages, content blocks, tools, and tool results; parse its SSE events and map stop reasons. Verify: a text fixture, a tool fixture, and a thinking fixture each produce the expected events.
- [x] 1.4 Add the Gemini adapter. Build `contents`, `systemInstruction`, and function declarations; parse candidates, function calls, and usage metadata. Verify: text and function-call fixtures produce the expected events and token counts.
- [x] 1.5 Gate capabilities. Send only accepted thinking and token fields, omit `search` from the schema and prompt when the provider lacks it, and keep cost unknown off DeepSeek. Verify: OpenAI has no `search` tool or prompt line, Gemini has no DeepSeek fields, and an unknown model shows no cost.
- [x] 1.6 Document providers. Add the `provider` setting, key variables, and examples to the README and `--help`. Verify: help lists `--provider`, and the README names each provider's key.
- [ ] 1.7 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
