# Tasks

- [x] 1.1 Put `../` first in directory listings. Insert the parent after sorting, omit it at `/`, and let Enter and Tab reuse the existing directory handling. Verify: the first row is `../`, Enter opens the parent, Tab selects it, and `/` and `/image` have no row.
- [x] 1.2 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
