# Design

## Request

`Session::suggest()` reuses `complete_once` with the history plus one system
message: "Write the user's most likely next message, 2 to 8 words, in their
voice. Reply with only that message, or with nothing if there is no clear next
step." It sets `max_tokens` to 24, temperature 0, and thinking off. The
history is the same prefix as the last request, so DeepSeek bills it at the
cached rate.

The reply is dropped when it is empty, longer than 60 characters, holds a
newline, or starts with a tool call. Surrounding quotes are stripped.

## When

The worker sends `Ev::Done` first, then asks for a suggestion, so the chat is
never kept waiting. It is skipped after an error, a cancel, a check-in turn,
or a goal round, and when the setting is off. The chat ignores a suggestion
that arrives after the user has typed or sent anything.

## Accepting

Tab or Right on an empty input fills it in and leaves the caret at the end.
Enter alone does not send it, so a stray Enter sends nothing by accident.
