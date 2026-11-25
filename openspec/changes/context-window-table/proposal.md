# Proposal: context window table entries

## Why

The status line shows the share of the context window in use, but only for models with a known
window. Today the table holds one model, `deepseek-flash`, so other built-in models show no percentage
unless the user sets `context_window` by hand.

## What Changes

- The window table gains `gpt-4.1-mini` (1,047,576 tokens) and `gemini-2.5-pro` (1,048,576 tokens). Each
  value is cited in the code with the published source.
- The configured `context_window` still wins over the table.

## Out of Scope

- `claude-opus-5-5`, Ollama and Custom models. No authoritative window figure was found for
  `claude-opus-5-5`, so it keeps using `context_window`. Ollama and Custom depend on the local model.
- Changing how the percentage is drawn or coloured.

## Impact

- `src/context.rs`: two table entries with source comments, and tests.
- `README.md`: the sentence that says which models show a percentage.
