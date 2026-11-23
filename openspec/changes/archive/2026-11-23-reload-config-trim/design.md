# Design

## Approaches

- **A sorted `Vec` with linear lookups (chosen).** A config has tens of keys;
  linear search costs nothing and pulls in no new generic code.
- **Keep the `BTreeMap`.** Clearer by a line, 16 KB larger.
