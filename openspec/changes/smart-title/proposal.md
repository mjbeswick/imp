# Proposal

## Why

A session is named by the first 60 characters of its first message, which for
a real prompt is a sentence fragment. `/rename` changes the stored title but
the title bar ignores it and keeps showing the first message.

## What Changes

- The immediate title is the first message cut at a word, at most 40
  characters, without a leading `please` or `can you`.
- After the first reply, a message longer than that gets a summary of at most
  six words from one small request.
- The request holds only the first message, never the history: about 150
  tokens in and 10 out, once per session.
- `title_summary = auto|on|off`; `auto` is on for hosted providers and off for
  Ollama and custom endpoints, where the request would take the local model.
- The title bar and the lists show the stored title, so `/rename` shows.
- A title set with `/rename` is never replaced.

## Keyboard and resources

No keys. One tiny request per long-first-message session, made off the UI
thread after the reply, and skipped when the user is already typing.
