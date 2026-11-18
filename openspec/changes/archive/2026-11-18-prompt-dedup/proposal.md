# Proposal

## Why

A confirmation repeats the call it is asking about. The tool row already names
the action, then the prompt prints the same command again:

```
⠹ run `nmap -sV -T4 ...`
run `nmap -sV -T4 ...`? y/N  (48s)
```

On a long command that is two wrapped copies of the same text, and the answer
hint gets pushed around to make room.

## What Changes

- The prompt asks with the verb alone: `run it? y/N`, `write it? y/N`,
  `save it? y/N`, `delete it? y/N`, `approve? y/N` when the verb is unknown.
- The tool row above stays the place that names the command, path, or note.
- The countdown and the row width stay as they are.
- Transcript lines after an answer keep the full text, so history is still
  specific: `declined: run \`nmap ...\``.

## Capabilities

### Modified Capabilities

- `tui`: a confirmation asks a short question instead of repeating the call.

## Impact

- `src/tui.rs`: prompt text.
- `openspec/specs/tui/spec.md`: confirmation requirement.
