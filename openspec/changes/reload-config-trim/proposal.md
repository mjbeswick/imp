# Proposal

## Why

`reload-config` added 16,576 bytes, nearly all from a `BTreeMap` used to diff
a handful of config keys. The README still says `/reload` re-reads only
instructions and skills.

## What Changes

- The diff uses a sorted `Vec`; behaviour is unchanged. Net cost of
  `reload-config` falls to about 32 bytes.
- The README's command list says `/reload` re-reads the config too.

## Keyboard and resources

16 KB smaller binary.
