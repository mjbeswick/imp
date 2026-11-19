# Proposal

## Why

The composer accepts text only. A terminal cannot paste pixels: it pastes a
path or nothing. Dragging a screenshot into the terminal therefore either
types a filename the model cannot open or does nothing. There is no `/image`
command and the wire format has no content parts.

## What Changes

- Accept an image path pasted or dragged into the composer on submit, and add
  `/image <path>` as the explicit form.
- Copy each image into `~/.local/state/imp/images/`, record it with the
  message, and reattach it on resume.
- Send images as OpenAI-style content parts after the text, with a size cap.
- On the first rejection, drop the image, retry the text alone once, and say
  the model refused the image.
- Render one dim marker line per attachment, and list `/image` in help and the
  README.

## Capabilities

### New Capabilities

- `images`: attaching image files, storing them with a session, the wire
  format, limits, rejection handling, and the transcript marker.

## Impact

- `src/api.rs`: `Message` gains attachments and emits content parts.
- `src/store.rs`: JSONL records gain an `images` array; image copies live under
  the state directory.
- `src/agent.rs`: reject-and-retry-once when the API refuses image content.
- `src/tui.rs`: `/image`, paste detection on submit, pending attachments, and
  the marker.
- `src/main.rs`: `/image` and command output in line mode.
- `src/help.rs` and `README.md`.
