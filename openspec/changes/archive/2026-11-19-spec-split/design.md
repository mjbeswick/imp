# Design

## Approaches

1. **Split by behaviour.** Chosen: each moved clause becomes its own
   requirement with one scenario, which is what the validator asks for.
2. **Trim words only.** Rejected: the overlay and help texts cannot fit 500
   characters without dropping behaviour.

## Decisions

- The shortened requirements keep every existing scenario, since a MODIFIED
  block replaces the whole requirement.
- New requirements: chat column spacing, the shared help column, the space
  rule in value filters, and the picker frame labels.

## Tests

- `openspec validate --all --strict` passes after archive.
