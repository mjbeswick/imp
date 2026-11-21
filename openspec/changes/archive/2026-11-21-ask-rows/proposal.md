# Proposal

## Why

An `ask` question and its choices are drawn on one row and cut at the screen
edge. In session 512a0def8, at 130 columns, the later choices of "Which state
machine should I chart?" were off screen, so the user could not read what the
number keys picked.

## What Changes

- An open question takes several rows above the rule: the question, one row
  per choice, and the key hint, each wrapped at the screen width.
- Confirmations and text prompts keep one row.

## Capabilities

### Modified Capabilities

- `tui`: pending questions are not cut.
