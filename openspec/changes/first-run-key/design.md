# Design

## Approaches

- **Provider picker and masked entry in the chat, key in a `0600` file
  (chosen).** The flow the request asks for, and the store is separate from
  the config people share and the agent may read.
- **Write `api_key` into the config.** No new file, but `init` explicitly says
  not to, and the config is the one file the agent is allowed to read.
- **OS keychain through `security` or `secret-tool`.** No new dependency, but
  two platform paths and a new failure mode for one key per provider.
  Rejected.
- **Read the key from stdin at startup.** No picker, so no provider choice,
  and it does nothing in a pipeline. Rejected.

## Resolution

One order, in `resolve_key_env`: `IMP_API_KEY`, then the profile's variable,
then the config `api_key`. The profile variable is looked up in the
environment and then in the keys file, so a stored key behaves like an
exported one and `Cmd::UseProvider` finds it without new plumbing.

The keys file is `$XDG_CONFIG_HOME/imp/keys`, one `NAME=value` line per
variable, `#` comments and blank lines skipped, written `0600`. Values are
trimmed; the first `=` splits, so a key may contain one.

## Startup

`main` computes whether this run is a chat before resolving. A missing key is
`Some(variable)` rather than an error: line mode returns it as
`{var} is not set; run imp in a terminal to enter a key`, and the chat passes
it to `tui::run`. Nothing else changes shape.

## Onboarding

The chat opens the provider picker with the active provider highlighted. The
choice is resolved to a profile the same way `set_provider` does today. A
profile with no key variable applies at once. Otherwise a masked row opens
under the composer naming the variable, and typed characters append, Backspace
removes.

Enter draws a checking row, calls `providers::list_models` with its five
second timeout, then decides:

- success: save the key, apply the provider, drop the row;
- `models: HTTP 4xx`: keep the row, show the server's answer, let the user
  edit;
- anything else: save the key and apply the provider, with a warning that the
  check did not run. A offline user is not locked out by a bad network.

The key is saved before the provider is applied, so the provider command finds
it in the store.

Esc quits. Nothing useful runs without a key, and a half-open chat would only
ask again on the first turn.

## Testing

The network call is the only untestable part. The decision is a pure function
of the check result, tested on its own: a 4xx retries, every other error
saves. The picker, the mask, the store, and the startup branch are tested as
they are.

## Size

Measured at 16.6 KB over the previous build. The first estimate of under
3 KB was wrong: the key row, the file handling, and their strings cost more
than a picker does.
