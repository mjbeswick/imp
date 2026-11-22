# Design

## Approaches

- **Move only the tests (chosen).** One mechanical move; code stays together.
- **Split the code into modules.** Larger diff and more files for no
  behaviour gain yet.

## Details

`tui.rs` becomes `src/tui.rs` plus `src/tui/tests.rs`; private items stay
reachable through `use super::*`.
