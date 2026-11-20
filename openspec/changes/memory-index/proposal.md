# Proposal

## Why

The model never reads saved notes. The prompt only says "search before
guessing", so a model asked about the weather has no reason to suspect a note
called `user-location-woking-uk` exists, and asks the user again.

## What Changes

- The system prompt lists notes, one line each: slug and title, capped at 40,
  like the skills index. Bodies still come only from a search.
- Memory search matches a note when every word of the query appears in its
  name or body, in any order, instead of the whole query as one phrase.
- The index is built with the prompt, so it refreshes on `/reload` and in
  every new session.

## Capabilities

### Modified Capabilities

- `memory`: a note index in the prompt, word-wise search.

## Impact

- `src/store.rs`, `src/prompt.rs`, `README.md`.
