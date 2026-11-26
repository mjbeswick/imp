# Design

## Context

Citations are to `main` at `uvxqvvls`, which holds the merged `composer-style` work.

- The `/compact` command arm in `src/tui.rs` (around line 5347) matches the exact word `compact`. It sends
  `Cmd::Compact` with no payload and pushes `compacting…`. A typed `/compact text` does not match that arm.
- The worker loop (around line 8574) calls `worker_session.compact(&mut hooks)` for `Cmd::Compact`.
- `Session::compact(&mut self, hooks)` in `src/agent.rs` (around line 886) builds a fixed prompt, with a 400
  token output cap, from the transcript, which takes the first 200 characters of each message. It replaces the
  summarised range with one system message named `compacted`.
- Automatic compaction calls the same function at around line 1053, when the estimate passes `compact_at`.
- The help line `("/compact", "compact the conversation now")` is in `src/help.rs` (around line 58).
- The spec for compaction is `openspec/specs/conversation-context/spec.md`, requirement "Compaction runs on an
  absolute token budget" (line 56). The tui command list is in `openspec/specs/tui/spec.md`.

## Decisions

1. **Argument.** `/compact` followed by text sends the text with the command. A command with no text is
   unchanged. Judgement call: the text is trimmed and empty text counts as no text.
2. **Prompt.** The focus text is added to the summary request as one extra line, such as
   `Focus: <text>`, placed before the transcript. The rest of the prompt is unchanged. Judgement call: the
   focus line goes before the transcript so the model reads it first.
3. **Cap.** The focus text is cut at 500 characters, and the reply line says `focus cut to 500 characters`.
   Judgement call: 500 characters bounds the prompt. Cost if wrong: long focus text is cut.
4. **Automatic compaction** passes no focus, so its behaviour does not change.
5. **Signature.** `Session::compact` takes an `Option<&str>` focus. The automatic call passes `None`. The
   existing tests pass `None`.
6. **Composer.** `/compact` with a space and text stays a command with free text, as the tui spec already says
   for commands without values. No picker opens.

## Risks

- Risk: focus text that asks for a summary of a different kind may change what the next model sees. The user
  asked for this.
- Risk: the focus text is free text in the prompt, so it can contain instructions to the summariser. The
  summariser's own role line stays first, and the output is still capped at 400 tokens.
- Assumption: the 500-character cap is enough for a focus line.

## Expected bytes

The change adds one argument parse, one prompt line, and one cap. The size change is measured with
`./scripts/size.sh` after implementation. No fixed figure is claimed here.
