# Design: context window table entries

## Context

- `WINDOWS` in `src/context.rs` (line 6) holds `(model name, tokens)` pairs. Its one entry,
  `deepseek-flash`, cites the DeepSeek pricing page in a comment.
- `window(model, configured)` returns the configured size when set, else the table entry, else nothing
  (`src/context.rs`, line 14). `percent` returns nothing for an unknown window (line 25).
- Sources checked when this change was written:
  - OpenAI GPT-4.1 mini model page, `developers.openai.com/api/docs/models/gpt-4.1-mini`: "1,047,576 context window".
  - Google Gemini 2.5 Pro documentation, Gemini Enterprise Agent Platform, 2-5-pro page: "Context window 1,048,576".

## Decisions

- **Two entries.** `("gpt-4.1-mini", 1_047_576)` and `("gemini-2.5-pro", 1_048_576)`. Model names match the
  names the built-in providers use (`src/providers/mod.rs`): OpenAI's default is `gpt-4.1-mini`, Gemini's is
  `gemini-2.5-pro`.
- **Source comment on each entry**, with the page and the figure, as `deepseek-flash` has.
- **No change to `window` or `percent`.** The lookup already does what is needed.

## Risks

- Providers may change a model's window. Each entry names its source so it can be rechecked. The
  configured `context_window` still overrides the table.

## Assumptions

- The model names in the table match the names the user sends. A model under another name gets no
  percentage, as before.
