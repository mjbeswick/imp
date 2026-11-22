# Design

## Approaches

- **Compare against the proposal commit (chosen).** jj finds the commit that
  added the change folder; `jj file show` gives the main spec at that point.
- **Store a hash of each modified requirement in the change.** Needs every
  proposal to record it; easy to forget.

## Details

For each `MODIFIED` requirement name, extract the block from the main spec
now and at the proposal commit. Different text means another change landed
in between: the script prints both and exits non-zero.
