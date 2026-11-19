# Tasks

- [ ] 1.1 Add the attachment model and storage. `Message.images`, copy into the state images directory with a content hash, deduplicate, and a human size helper. Verify: a PNG copies once, a second attach reuses it, and a missing or unsupported file is refused.
- [ ] 1.2 Send content parts. Emit a text part and one `image_url` data URL per image, and keep a plain string when there are none. Verify: a JSON test checks both shapes and the `data:image/png;base64,` prefix.
- [ ] 1.3 Add `/image` and paste detection. Queue attachments on the session, apply the 10 MB and four-image limits, strip pasted paths from the text, and push the marker line. Verify: `/image` queues, a pasted path attaches, an oversized file is refused, and the marker reads `[image name · N KB]`.
- [ ] 1.4 Add rejection handling. On a 400 that names image content, drop the attachments from the last user message, retry once, and report it; do not retry a second time. Verify: a stub transport returns the rejection, then succeeds text-only, and the retry count is one.
- [ ] 1.5 Wire line mode and docs. `/image` in line mode prints the marker and attaches to the next turn; add the command to help and the README. Verify: a piped `/image` run shows the marker, and `/help` lists thirteen commands.
- [ ] 1.6 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
