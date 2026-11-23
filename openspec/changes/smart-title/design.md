# Design

## Approaches

- **Cut first, summarise once, only when needed (chosen).** The cut title is
  free and instant; a short prompt is already its own title, so only long ones
  pay for a request.
- **Summarise every first message.** Spends a request on prompts that need
  none.
- **Ask the main model to start its reply with a title.** No extra request,
  but it pollutes the reply and the prompt.
- **Piggyback on the next-message suggestion.** That request carries the whole
  history; a title needs only the first message.

## Details

The worker sends the request after `Ev::Done`, as it does for suggestions,
but first drains one waiting command into its queue and skips the title if
there was one, so the user's next message is never held behind it. The reply
becomes `Ev::Title`; the chat keeps a `title` that `/rename` and `Ev::Title`
set and the title bar shows it, falling back to the cut first message. The
worker applies it with the existing `Log::retitle`, unless the sidecar title
no longer equals the cut one, which means the user renamed it.

Cost on DeepSeek is about $0.00005 per session.
