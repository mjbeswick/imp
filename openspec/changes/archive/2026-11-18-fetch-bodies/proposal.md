# Proposal

## Why

A news question cost three `curl` calls and three approval prompts. `fetch`
does return the page, but it spends its first line on a heading outline, so a
read of a markdown file starts with `outline: Steno Mode — ...` and the model
reaches for the shell to get the body. Each `curl` is a permission prompt and a
shell with network authority.

## What Changes

- Drop the heading outline from a `fetch` result. The title and body stay, under
  the existing 4 KB cap and spill rule.
- Tell the model in the baseline prompt to fetch a URL with `search`, to use
  `curl` only for a plain-text endpoint or an authenticated call, and never to
  scrape with `curl | grep | sed`.

## Capabilities

### Modified Capabilities

- `tools`: a fetch result is the page, with no outline prefix.

## Impact

- `src/agent.rs`: the fetch arm and one prompt line.
- `src/prompt.rs`: the fetch rule.
- Tests for both.
