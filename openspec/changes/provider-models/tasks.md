# Tasks

## 1. Config

- [x] 1.1 `provider.<name>.models` parses as a comma-separated list and is written back, with the line omitted when empty; tests
- [x] 1.2 The default model is always included when the provider is saved; tests

## 2. Form

- [x] 2.1 The provider form has a `models` field; its value is split on commas, trimmed, and saved; tests

## 3. Model picker

- [x] 3.1 `/model` lists every model of every configured provider, each row showing its provider; tests
- [x] 3.2 Choosing a row switches the session to that provider and model and shows the provider on the status line; tests

## 4. Docs

- [x] 4.1 README's provider section describes the `models` field and the `/model` rows
