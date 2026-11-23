# Proposal

## Why

Every Anthropic request carries a temperature: 0.2 for turns and 0.0 for
summaries and suggestions. Current Claude models reject sampling parameters
with a 400, so choosing one with `/model` fails every request. The built-in
catalog lists only older models, which is why nobody has hit it yet.

## What Changes

- Anthropic requests send no `temperature`. Other providers are unchanged.
- The Anthropic catalog lists `claude-opus-5-5`, `claude-sonnet-5-5` and
  `claude-haiku-5-5`; the provider default becomes `claude-opus-5-5`.

## Keyboard and resources

No keys change. No cost on the device.
