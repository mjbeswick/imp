# Proposal

## Why

Tool output loses every escape sequence, so `chafa` pictures and coloured
charts arrive as bare blocks, and a successful command's output is seen only
when expanded. Diagrams the model writes as Mermaid stay as source.

## What Changes

- Output shown to the user keeps colour and text style (SGR) and nothing
  else: cursor moves, clears, titles, and image protocols are dropped. Rows
  are cut to the screen width with their colours intact.
- Commands typed with `!` show their output in colour.
- `bash` takes `show`: the output, up to 200 lines, is shown to the user in
  colour. The model gets the plain text, or for a picture made of block or
  braille characters a placeholder such as `[shown to the user: a 60×24
  picture]`.
- `render.<lang> = <command>` in the config renders a closed fenced block of
  that language in replies through the command: the block's text on stdin,
  `COLUMNS` set to the width, two seconds at most, output shown in place of
  the block. A failure shows the block as code. Each result is cached, and
  the conversation keeps the source.
- With colour off, all of it is plain text.

## Capabilities

### Modified Capabilities

- `tui`: coloured output and rendered blocks.
- `tools`: the agent can show output.

## Impact

- `src/render.rs`, `src/tui.rs`, `src/agent.rs`, `src/main.rs`,
  `src/config.rs`, `src/prompt.rs`, `README.md`.
