# Design

## Approaches

- **Never send temperature to Anthropic (chosen).** Correct for every
  current model; older models fall back to their default sampling, a small
  change from 0.2.
- **Send it only to models known to accept it.** A model list to keep in
  step with Anthropic's releases.

## Details

The adapter drops `opts.temperature`. The catalog and default change in
`providers/mod.rs`. Thinking and effort for Anthropic are a separate change.
