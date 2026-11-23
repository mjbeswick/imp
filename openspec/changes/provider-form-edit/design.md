# Design

## Context

The form is `Pending::ProviderForm { name, fields, at }` in `src/tui.rs`. It
is opened from `/provider add` (around line 4590), from `/provider edit`
(around line 4608), and from first-run setup (around lines 4807 and 4820). The
key handler is in the same file, where Enter increments `at`, Backspace pops
from `fields[at]`, and every other key is ignored.

## Decision

Add ↑, ↓ and Shift+Tab (`Key::BackTab`) to the handler: they change `at`
within bounds and keep `fields` as they are. Nothing else changes, so the
first-run path gets the same behaviour from the same code.

## Risks

- Shift+Tab is the mode key elsewhere. It only changes meaning while a provider
  form is open, which is an existing pattern for pickers.
