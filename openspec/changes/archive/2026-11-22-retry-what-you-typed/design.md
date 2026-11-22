# Design

## Approaches

- **The chat names the text (chosen).** The transcript is what the user sees,
  so it decides what "last message" means.
- **The agent names it and the chat follows an event.** Needs a new event and
  pending state in the chat for the same result.

## Details

The chat drops its rows at once. When the agent cannot find the text, it says
so and the transcript stays shorter than the history until the next turn;
compaction is the only way there, and it is rare.
