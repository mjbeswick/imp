# Design

## Context

`jj` has no commit hooks and this machine has no CI. A hook is therefore not
available; the gate has to be a script that is cheap to run and run often.

## Goals / Non-Goals

**Goals:** catch ad-hoc edits in one command; give a clear reason; cost under a second.

**Non-Goals:** prevent a determined human from bypassing it; police editors.

## Decisions

**Path-based drift detection, not diffing content.** The check asks two questions:
did anything outside `openspec/` change, and did anything inside it change in the
same working copy. That needs no parsing of diffs, no baseline commit, and no
network.

**`skip_specs: true` for tooling.** These files change no runtime behaviour, so a
spec delta would be invented. Recorded in `.openspec.yaml`.

**One entry point.** `scripts/check.sh` is what a human or agent runs; CI would
call the same file. Multiple gates with different names is how gates get skipped.

## Decisions dropped

Two tasks proposed verifying that `opencode.jsonc` refuses edits to `target/**`
and that the `spec` agent cannot write source. They are dropped: permissions are
static and cannot express "only with an active change", so the config is
documented as guardrails and the stateful rule stays in AGENTS.md plus the
drift check. Verifying static config in a headless run would prove nothing about
the case that matters.

## Risks / Trade-offs

- Allows a change to be ticked without really being done → mitigated by the
  tasks stating their own verification, and by review.
- Path-based detection flags `README` edits as code → deliberate; documentation
  belongs to a change too.
