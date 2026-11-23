# Design

## Context

Evidence from the current code:

- `src/tui.rs:6606` `status_line` builds the left part as `{model} · {mode} {tokens} tok{money}…` using `compact_tokens(self.status.tokens)`.
- `src/tui.rs:7175` `context_usage` sets the context estimate after each request, sent as `Ui::Usage { context }` (`src/agent.rs:722`).
- `src/api.rs:375` `estimate_tokens` is a byte-based estimate. It is an estimate, so the percentage is an estimate too.
- `src/config.rs:120` `Config` holds the model and provider. `src/config.rs:286` shows the pattern for a numeric key (`max_tokens`, parsed with `parse_u32`).
- No context-window size exists anywhere in the code today. This is a new fact the change must add.

## Decisions

- **Percent = `min(100, tokens * 100 / window)`**, integer division, shown as `NN%`. Rounding down keeps a full-looking 100% from appearing before the window is actually full.
- **Window lookup order:** the `context_window` config key first, then the per-model table, then unknown. An unknown result omits the percentage.
- **Table:** a `const` slice of `(model prefix, tokens)` in a new `src/context.rs`. Only entries with a source are added. Each entry carries a comment naming its source. Models not verified are left to the config key.
- **Placement:** `12.3K tok (6%)`, so the cost and goal fields keep their existing positions.

## Assumptions

- A: the provider's reported window matches the model's documented window. Not verified here; the config override exists so a user can correct a wrong table entry.
- B: the byte-based estimate is close enough that a percent is useful. Not measured; the spec states that the figure is an estimate.

## Risks

- A wrong table entry shows a wrong percent. Mitigation: each entry cites its source, and the config key overrides it.
- Width: the extra `(6%)` lengthens the left part on narrow terminals. The existing `fit` truncation already handles this.
