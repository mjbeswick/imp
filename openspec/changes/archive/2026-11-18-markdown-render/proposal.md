# Proposal

## Why

Model replies arrive as markdown and are drawn as raw text: `**bold**`, fences,
lists, and `[text](url)` links all appear literally, and a URL is a dead string
in a terminal that could open it.

## What Changes

- Render assistant text as a small markdown subset: headings, bold, italic,
  inline code, fenced code, bullet and numbered lists, blockquotes, rules.
- Turn `[text](url)` and bare `https://` URLs into OSC 8 hyperlinks, drawn as
  the link text, in terminals that support them.
- Detect link support from the environment, with a `links=auto|always|never`
  setting and `IMP_LINKS` override.
- Keep streaming safe: a line renders when complete, the line still arriving
  stays plain, so a half-written link never emits a broken sequence.
- Wrap on styled spans and open one OSC 8 sequence per wrapped row, so no
  escape sequence is ever split by a line break.
- No new dependency; it stays inside the size budget.

## Capabilities

### Modified Capabilities

- `tui`: assistant text renders as markdown, and links are clickable where the
  terminal supports them.
- `setup`: a `links` setting selects link rendering.

## Impact

- New `src/render.rs` with the subset parser and span wrapping.
- `src/tui.rs`: rows built from spans.
- `src/config.rs`, `src/init.rs`, `src/help.rs`: the setting.
- Spec deltas under `tui` and `setup`.
