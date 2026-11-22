# Design

## Approaches

- **Prompt plus a cleanup (chosen).** The prompt asks for `?`; a short
  question-word check catches replies that still miss it.
- **Prompt only.** A 24-token reply at temperature 0 still drops it often.
  Rejected.

## Cleanup

`clean_suggestion` lowercases the first word and checks it against a short
list: what, why, how, where, when, which, who, can, could, should, would,
will, is, are, do, does, did, shall. A match without a closing `?` loses a
trailing `.` and gains `?`. The list misses questions that open otherwise,
such as `any other ideas`; the prompt covers those.

The composer drops the `  Tab` suffix.

## Size

Expected: under 200 bytes.
