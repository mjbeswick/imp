# Design: Question tool parity

## Context and evidence

`src/agent.rs` defines the single-question tool schema and parses options in
`ask_options`; `run_tool` returns a plain string. `Hooks::ask_choices` falls
back to labels alone (`src/agent.rs:39-47`). The TUI owns a question in
`Pending::Question` and handles numbers, typing, selection, and Esc in
`src/tui.rs`. `Cli::ask` only prints labels and accepts one number or line
(`src/main.rs`). The agent executes all tool calls in a response before
checking cancellation again (`src/agent.rs:1052-1055`). Existing question
scenarios live in `openspec/specs/tools/spec.md` and
`openspec/specs/tui/spec.md`.

## Decisions

- Extend `ask`, rather than duplicate it with another tool. A `questions`
  array takes precedence over the legacy `question` and `options` fields.
  Legacy calls keep their string result, including comma-joined multiple
  selection. New calls return a JSON object with `answers`, an array of
  arrays of exact chosen labels or a single custom text value per question.
- The `Hooks` boundary accepts a sequence of questions and returns a
  sequence of selections. Its default adapter handles simple front ends;
  TUI and line mode implement multiple selection without splitting labels
  on punctuation. Display progress and headers in the TUI's existing
  question rows rather than introducing a second modal.
- Empty and invalid question sets are rejected before opening a prompt.
  Cancelling any question dismisses the entire request. A dismissed prompt
  returns its refusal result but suppresses later tool calls in that model
  response; the agent then ends the turn.
- Yes/No keyboard shortcuts recognize an optional ` (Recommended)` suffix
  on the Yes or No label, without changing the returned exact label.
- The custom-answer caret belongs to its session's question. Store and swap
  its byte offset with the pane, and guard UTF-8 boundaries at rendering
  and editing so switching sessions cannot split a multibyte character.

## Risks and validation

Pausing across questions can expose a partial result; no partial selections
are returned on dismissal. Tests exercise schema/validation, structured
results and legacy output, keyboard and custom answers, line mode parsing,
and an `ask` followed by another tool call in the same response.
The fetched `main` already fails `composer_rows_fit_the_terminal` (91 `x`
characters vs 90 in the draft); diagnose and correct its assertion before
accepting the full suite. No unrelated rendering change is intended.
Review identified a second question-specific risk: the shared `ask_caret`
is not swapped with session panes in `src/tui.rs:2681-2691`; an offset from
another answer may fall inside a UTF-8 character. Test both pane switching
and subsequent editing with non-ASCII answers.

Expected release binary change: +2,000 to +12,000 bytes. Run
`./scripts/size.sh` before each commit and compare the final result.
