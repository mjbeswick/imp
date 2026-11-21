# Proposal

## Why

Showing a web image takes a download, and every download asks. Writing to
/tmp is no fix: it is shared with other users and programs, and on the Pi it
is a 1 GB tmpfs on a 463 MB machine.

## What Changes

- Each imp process gets a private scratch folder under its state directory,
  passed to agent commands as `IMP_TMP` and named in the prompt.
- read, write, and edit reach it without asking, as they already reach the
  state directory.
- A `curl -o` or `wget -O` download of one URL into `$IMP_TMP` is approved as
  a fetch, so it does not ask in normal mode.
- `chafa` joins the safe list: it only reads files and draws.
- The folder is removed when imp exits; at start, folders of dead processes
  are removed.

## Capabilities

### Modified Capabilities

- `permissions`: scratch downloads and chafa run unprompted.
- `tools`: agent commands see `IMP_TMP`.
