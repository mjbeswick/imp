# Design

## Approaches

- **Carry the estimate on the usage event (chosen).** The agent already holds
  the history when it counts the request, so it can send the context with it.
  No new event, no extra request.
- **Estimate in the app on every frame.** The app has no history; it would be
  a guess from counters that are not the context.
- **Estimate only on a timer.** Stale by construction.

## Details

`Ui::Usage` gains `context`, filled with `api::estimate_tokens(&self.history)`
at the same point as the billing counters. `WorkerHooks` maps it to
`Ev::Usage { tokens: Some(context), .. }`, which the app already applies.

The status line formats cost as `$0.001` below 0.1 and `$0.42` above, the same
rule the task line uses.

## Size

Expected: under 300 bytes.
