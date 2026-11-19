# Design

## Approaches

1. **Fetch on the worker, catalog first.** Chosen: the picker opens at once
   with a catalog, the worker GETs the provider list, and the open picker
   updates. No frozen frames and no new dependency.
2. **Fetch on the UI thread.** Blocks input for the request timeout and freezes
   the spinner.
3. **Catalog only.** Simple, but wrong as soon as the provider adds a model.

## Decisions

- `http::get(url, headers, timeout)` reuses the libcurl setup behind the
  existing POST, without a body and with a five-second limit. The body is
  collected as a `String`.
- `providers::catalog(provider)` returns a short curated list per built-in
  provider; custom profiles return an empty list because their names are
  unknown.
- `providers::list_models(provider, base, key)` chooses an endpoint:

  - DeepSeek, OpenAI, Ollama, custom: `{base}/models` with a Bearer key.
  - Anthropic: `{base}/v1/models` with `x-api-key` and `anthropic-version`.
  - Gemini: `{base}/v1beta/models` with `x-goog-api-key`.

  Responses are `data[].id` except Gemini, whose entries are `models[].name`
  with the `models/` prefix removed. The result is sorted and deduped.
- `Cmd::ListModels` carries the provider, base URL, and key as they are at the
  moment of the request, so a provider switch cannot race the response. The
  worker sends `Ev::Models(Vec<String>)`; on error it sends the catalog.
- `App` keeps `available_models`, seeded from the catalog and replaced by the
  event. `model_items` builds current, default, available, and recents in that
  order, deduped, with `(current)` and `(default)` labels.
- `Ev::Models` updates the open Model picker in place, preserving the filter
  and clamping the index. A provider switch resets `available_models` to the
  new catalog.
- Tests: parsing each provider shape against fixed JSON, catalog contents,
  model item order and labels, live-list replacement, and fallback behaviour.

## Tests

- `list_models` parsing is tested without network through a `parse_models`
  helper per shape.
- `model_items` lists current, default, catalog, and recents without duplicates.
- An `Ev::Models` event replaces the entries of an open model picker.
- A failed fetch still leaves the catalog rows.
