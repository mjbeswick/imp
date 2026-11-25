# Proposal: Question tool parity

## Why

`ask` can only ask one question and returns an unstructured string. The TUI
offers described and multiple-choice options, but line mode silently drops
both. Dismissing a question must also prevent later calls in the same model
response from executing. These gaps make the question tool less useful and
less safe than OpenCode's question flow.

## What Changes

- Accept an ordered `questions` array with a short header, described choices,
  and optional multiple selection per question.
- Return a structured answer per question, retaining the existing single
  question input and string result for older callers.
- Present and answer the questions in order in both TUI and line mode, with
  custom text and dismissal of the whole request.
- Stop executing subsequent tool calls when a question is dismissed; make the
  recommended Yes/No choice answerable by the existing keyboard shortcuts.

## Out of Scope

No new remote service, provider API, or change to permission confirmations.

## Impact

`src/agent.rs`, `src/tui.rs`, `src/main.rs`, their tests, and the question
tool schema. The existing `ask` name and legacy call shape remain supported.
