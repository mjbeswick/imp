# Proposal

## Why

Bare `/model` shows one row. The picker only offers the current model, the
provider default, and recents; because the default equals the current model,
dedupe leaves one row. The user cannot see the models the provider actually
offers.

## What Changes

- Bare `/model` lists the provider's models, fetched from its models endpoint,
  alongside the current model, the provider default, and recents.
- A built-in catalog per provider fills the picker immediately and stands in
  when the fetch fails.
- The fetch runs on the worker thread with a short timeout, so the interface
  never blocks; the open picker updates when the list arrives.
- Switching provider swaps the catalog and clears the fetched list.
- `/model <name>` still accepts any name and keeps working without a fetch.

## Capabilities

### Modified Capabilities

- `tui`: the model picker covers the provider's models with a catalog
  fallback.

## Impact

- `src/http.rs`: one GET request beside the existing POST.
- `src/providers/mod.rs`: catalog, endpoint, and response parsing per provider.
- `src/tui.rs`, `src/agent.rs`: list command and event.
- `openspec/specs/tui/spec.md`, `README.md`.
