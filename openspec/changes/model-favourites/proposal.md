# Proposal: model favourites

## Why

The `/model` picker lists the active provider's models, the models of every usable provider, and
the current model. A user who switches between a few models has to scan the whole list each time.
The user wants to mark the models they use most as favourites, so they stay at the top of the picker
across sessions.

## What Changes

- In the `/model` picker, pressing `f` on a model row toggles that row as a favourite. The key works
  only while the filter is empty, as the other row actions do.
- Favourites are stored in a file next to the recent-models file, one `provider::model` value per
  line, so they survive restarts.
- The model picker lists favourites first, in catalog order, each marked with a star. Other rows keep
  their current order.
- Toggling a favourite keeps the highlight on the same row and shows a toast saying whether the model
  is now a favourite or not.

## Out of Scope

- Favourites in the provider picker, the `/model <name>` command-line form, and the provider-model
  picker opened from the provider form.
- Changing which models a provider offers, or how the model catalog is fetched.
- Syncing favourites between machines or writing them into `config.toml`.

## Impact

- `src/tui.rs`: favourite file helpers beside `load_models` and `record_model`, `model_items` ordering
  and star marker, and the `f` action in the picker key handler.
- `src/tui/tests.rs`: tests for toggling, persistence, ordering, the filter guard, and the star marker.
- `openspec/specs/picker/spec.md`: a new requirement, delivered by this change's delta spec.
