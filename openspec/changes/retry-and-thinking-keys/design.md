# Design

## Approaches

- **Drop the reply, then resend (chosen).** The history reads as if the bad
  reply never happened, so the model is not steered by it.
- **Resend and keep the old reply.** The model sees its own failed answer;
  that is what `/fork` plus an edit is for.

## Details

A new `Cmd::Retry` asks the agent to cut the history after the last user
message and run the turn again. The transcript drops the same rows. Files the
old turn edited stay edited; `/diff` still shows them. Retry is refused while
a turn runs, and with no earlier message.
