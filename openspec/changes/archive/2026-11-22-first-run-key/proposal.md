# Proposal

## Why

imp demands the provider's environment variable before the chat opens.
`resolve_key_env` runs at startup and exits with `DEEPSEEK_API_KEY is not set`,
so a new user with a key in hand cannot get in without editing a shell
profile. The config has an `api_key` field, but `init` tells people not to use
it. There is no path that asks for a key.

## What Changes

- A key resolves from the environment first, then `~/.config/imp/keys`
  (`0600`), then the config `api_key`.
- A chat started without a key opens a provider picker, then a masked key
  row. Enter checks the key, saves it, and starts the chat.
- Ollama and custom hosts need no key and skip that row.
- Line mode and pipes still refuse, and now name the fix.
- `/provider use` reads the same store, so a chosen provider picks up its
  saved key.

## Keyboard and resources

The picker keeps its keys. The key row takes characters, Backspace, Enter, and
Esc. One request of at most five seconds on Enter, one small file.

## Capabilities

### Modified Capabilities

- `providers`: where a key comes from, and first-run entry.
