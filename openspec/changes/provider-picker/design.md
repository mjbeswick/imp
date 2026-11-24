# Design: provider picker

## Context

Evidence from `src/tui.rs` (line numbers at the time of writing):

- `picker_row` (2192) wraps a description to two lines only when `wrap` is true and the
  row is selected. `picker_lines` (2248) passes `wrap && selected`, so the selected row is
  the only one that can grow. That is the height jump.
- The overlay (around 6899) calls `picker_lines` with `wrap = true`, then falls back to
  `false` on a short screen. The fallback stays.
- The Enter branch for `PickKind::Provider` (around 6070) calls `configure_provider` when
  not onboarding, with the comment "Enter opens the form; switching is `u`". The
  onboarding branch above it calls `choose_onboard_provider`, which is not touched.
- Observed in a terminal session: Enter on `custom` printed `provider: custom` and switched
  the active provider at runtime. The config file was not written. The exact key path
  that reaches `use_provider` is not yet identified. The first task is a failing test
  that reproduces it.

## Decisions

1. **Stable row layout.** `picker_lines` wraps a provider row whenever `wrap` is true,
   not only when it is selected. Other picker kinds keep the current rule.
2. **Enter opens the editor.** The provider Enter branch calls `configure_provider` for
   every provider row. The `u` key stays on `use_provider`.
3. **Test first.** A test drives the picker to `custom` and presses Enter. It asserts the
   pending state is `ProviderForm` and that the active provider is unchanged.

## Risks

- A provider list with long descriptions takes two lines per row on every row, not only
  the selected one. The list therefore needs more height. The short-screen fallback
  already covers small terminals.

## Expected size

Under 30 lines of Rust in `src/tui.rs`, plus tests. No binary size concern.
