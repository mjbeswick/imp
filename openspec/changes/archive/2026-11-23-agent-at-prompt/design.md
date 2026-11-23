# Design

## Approaches

- **Name before the chevron, removed elsewhere (chosen).** One place, the one
  the eye is on.
- **Keep the status line and add the prompt.** Three places for one fact.

## Details

The prompt prefix is `{agent} {glyph} ` when more than one agent exists,
using the coming agent when one is set. Continuation rows indent by the
prefix width so the draft stays aligned. Scenarios that read the status line
for the agent move to the prompt.
