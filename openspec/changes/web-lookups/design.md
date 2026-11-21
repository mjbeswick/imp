# Design

## Decisions

- **No host list for GETs.** The prompt protected against a URL carrying data
  out. A long URL is the signal that matters, so only that asks, every time.
  Secrets are already stripped and redacted by `env-secrets`.
- **Date in the composed prompt,** not the baseline: the baseline has a byte
  budget, and the date changes the cached prefix only once a day.
- **Markdown by walking tags,** as `html_text` already does, emitting `#` for
  headings, `[text](url)` for links with absolute URLs, `-` for list items, and
  `|` rows for tables. Relative links resolve against the page URL. No parser
  dependency.
- **Feeds by tag name.** `<item>` and `<entry>` with `title`, `link` or
  `link href`, and `pubDate`, `updated`, or `published`. CDATA is unwrapped.
- **A gist, not a new field.** A successful step's `preview` becomes a short
  gist per tool; a failure keeps its reason. Both front-ends use it.
- **`Verbosity` enum** in the config, default `medium`, read at start and
  changed per session with `/verbosity`.

## Keyboard

No change. `/verbosity` gains `medium`; Ctrl+E still expands one row.

## Resources

No new process or dependency. The markdown walker replaces the text walker.
