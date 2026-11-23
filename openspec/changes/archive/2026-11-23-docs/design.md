# Design

## Approaches

- **Two guides beside the README (chosen).** The README stays the reference;
  the guides explain. Each claim is checked against the code and the specs.
- **Split the README into many reference pages.** More files to keep in step
  for no gain yet.

## Details

The agent loop guide names the constants that shape a turn (24 steps, prune
at 45,000 tokens, compact at 60,000, five retries) and points at the code
and specs that own them, so it can be checked when they change.
