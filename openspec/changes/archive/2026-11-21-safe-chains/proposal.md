# Proposal

## Why

After a scratch download, `file x; ls -l x` still asked: any chained command
asks, even when every part is read-only. Models chain constantly, often as
`cd dir && ls`, so most remaining prompts are for safe work.

## What Changes

- Commands joined by `;`, `&&`, or `||` run unprompted when every part passes
  the existing safe-pipeline check. A lone `&`, an empty part, a redirect, or
  a substitution still asks.
- `cd` joins the safe list; it only moves the rest of the chain.

## Capabilities

### Modified Capabilities

- `permissions`: safe chains run unprompted.
