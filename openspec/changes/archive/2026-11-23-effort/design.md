# Design

## Approaches

- **A separate dial (chosen).** Matches the APIs: Anthropic's effort applies
  even without thinking, and newer Claude models cannot turn thinking off.
- **One control with an `off` level.** Breaks on providers where the two are
  independent.

## Details

`RequestOptions` gains `effort: Option<Effort>`. `Thinking::DeepSeek` and
`Thinking::Effort` map it to `reasoning_effort`, defaulting to `low` when
unset. The Anthropic adapter writes `output_config.effort`. Gemini and Ollama
have no dial here; the session notes it the first time effort is set.
