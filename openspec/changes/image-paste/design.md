# Design

## Approaches

1. **Copy on attach, reference by file name.** Chosen: the transcript stays
   small, resume works, and the copy survives the original moving. The cost is
   a small state directory and a cleanup rule later.
2. **Embed base64 in the JSONL.** Self-contained, but every resume parses
   megabytes and the log grows without bound.
3. **Reference the original path.** Cheapest, but a screenshot in `/tmp`
   breaks the session on resume.

## Decisions

- `Message` gains `images: Vec<Attachment>` with `file`, `mime`, and `bytes`.
  Old records deserialize with an empty list; records with no images serialize
  without the field.
- Copies live in `~/.local/state/imp/images/`. The name is a 64-bit FNV-1a hash
  of the bytes plus the extension, so the same image attaches once. Writes go
  to a temporary file and rename into place.
- Accepted types are PNG, JPEG, GIF, and WebP, checked by extension and magic
  bytes. Limits are 10 MB per file and four images per message. A refusal never
  changes the composer or the pending list.
- `Message::to_json` keeps a plain string when there are no images. With
  images it emits a text part then one `image_url` part per image, reading the
  copy and base64 encoding it at send time. A missing copy becomes a tool-level
  error naming the file, not a panic.
- `Session::attach_image(path)` validates, copies, and pushes onto
  `Session::attachments`. `turn` drains that list into the user message and
  clears it. Both front-ends use the same method, so `/image` behaves alike.
- On submit, the interface scans whitespace-separated tokens for existing
  images, strips the ones it attaches, and uses `Look at the attached image.`
  when nothing remains. Tokens with quotes or spaces are left to `/image`,
  which treats everything after the command as the path.
- The marker is a dim line pushed when the attachment is queued:
  `[image shot.png · 117 KB]`. Human sizes use B, KB, and MB.
- In chat mode the marker is an `Ev::Info`-style dim row. In line mode the same
  line is printed before the request.
- Rejection handling sits beside the existing retry-once path in the agent: a
  response whose status is 400 and whose body names image content removes the
  attachments from the last user message, then resends once. The transcript
  reports the drop. A second failure is reported without further retries.
- Sessions do not replay old rows on resume, so markers cover messages sent in
  the current run; the attachments themselves still travel on resume.
