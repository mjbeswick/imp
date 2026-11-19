# Tasks

- [x] 1.1 Add the detail field and builder. Extend the tool-end event, build `-`/`+` lines from edit pairs and write content, and cap at 200 lines or 8 KB with a cut note. Verify: an edit with two pairs renders removed then added lines, a write renders additions, and a 500-line edit stops at 200.
- [x] 1.2 Render the preview and colours. Store the full detail on the tool row, push a 12-line preview, paint `+` green, `-` red, `@@` info, and keep prefixes when colour is off. Verify: a 20-line change shows 12 lines plus `+8`, and the frame contains the add and delete colours.
- [x] 1.3 Expand with Ctrl+O. Replace the preview with the full detail when the last row expands, and restore the preview on collapse. Verify: the expanded frame contains line 20, and the collapsed one does not.
- [x] 1.4 Wire line mode. Print the detail under the row, coloured only when stdout is a terminal. Verify: a piped run contains `-` and `+` lines with no escape bytes.
- [x] 1.5 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
