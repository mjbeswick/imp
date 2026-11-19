# Design

## Approaches

1. **One rule in the key router.** Chosen: the input is the only thing that
   decides, so there is no per-mode table to keep in sync.
2. **A confirmation before quitting.** Adds a keystroke to the escape hatch and
   still loses the draft.
3. **A separate clear key.** Ctrl+L already clears the transcript, and the user
   asked for Ctrl+C to behave this way.

## Decisions

- `handle_key` runs the Ctrl+C rule before the pending dispatch. The rule is:
  - A pending confirmation or `ask` question exits. Those prompts do not use
    the composer, so there is no draft to protect.
  - Otherwise, if `input` is not empty, clear it, reset the cursor and history
    position, close any picker, and redraw.
  - Otherwise set `quit`.
- Clearing the composer also closes the picker. Pending attachments are
  separate and stay, since Ctrl+C acts only on text.
- The turn is untouched when text exists; clearing a draft is not a cancel.
  Esc remains the turn-cancel key.
- Tests: idle draft clears; empty exits; streaming draft clears without
  cancelling; streaming empty exits; picker with text closes and stays; picker
  empty exits.
- The help key line changes from `Ctrl+C quit` to `Ctrl+C clear or quit`.

## Tests

- `Ctrl+C` on `draft` leaves `input` empty and `quit` false.
- `Ctrl+C` on an empty composer sets `quit`.
- Streaming with a draft leaves `cancel` false.
- A picker and text are both gone after one press.
