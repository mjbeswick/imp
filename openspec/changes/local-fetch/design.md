# Design

## Approaches

1. **A local GET tool with one approval per host.** Chosen: it covers APIs,
   feeds, and the local network for every provider, and keeps a check on what
   leaves the machine.
2. **Safe-list read-only `curl`.** Rejected: telling a harmless `curl` from one
   that posts, uploads, or writes files means parsing its flags, and pipes
   still need judging.
3. **A prompt-free fetch.** Rejected: the `curl` prompt is today the only check
   that stops the model sending data out inside a URL.

## Decisions

- `fetch { url }` calls `http::get` with a 15 second limit and a
  `User-Agent: imp`. Only `http://` and `https://` URLs are accepted; anything
  else is refused before a request is made.
- `http::get` also returns the `Content-Type` header, read through libcurl's
  `CURLINFO_CONTENT_TYPE` after the transfer.
- The result starts with the status line, `200 application/json`, then the
  body. A status of 400 or above is a failed result with the body as its
  reason.
- HTML, by `Content-Type` or a leading `<`, is reduced to text: the `<title>`
  first, `script`, `style`, and `noscript` dropped, block tags turned into
  line breaks, the remaining tags removed, entities `&amp; &lt; &gt; &quot;
  &#39; &nbsp;` decoded, and runs of blank lines collapsed.
- Bodies go through the existing 4 KB cap and spill buffer.
- Approval: `Session` keeps the hosts approved this session. `perm` decides
  per mode: `yolo` allows, `strict` asks every time, `normal` asks for a host
  not yet approved. A refusal is a tool result, as for any tool.
- The deny list does not apply to URLs; GET cannot run code.
- `search` drops its `fetch` operation and the DeepSeek fetch endpoint call.
- `scripts/size.sh` sets `LIMIT=776000`; `AGENTS.md` names the same figure.

## Risks

- HTML reduced locally reads worse than DeepSeek's server-side markdown for
  long articles. Pages built by scripts give little text either way.

## Tests

- A refused scheme (`file://`) makes no request.
- HTML reduction: title first, scripts dropped, entities decoded.
- Approval: normal asks once per host and not again; strict asks twice; yolo
  never.
- The tool list carries `fetch` for every provider, and `search` has no fetch
  operation.
- A local HTTP server in a test answers JSON, and the result carries the status
  line and body.
