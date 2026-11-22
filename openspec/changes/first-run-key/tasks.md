# Tasks

## 1. Store and resolution

- [x] 1.1 Load and save the keys file at `$XDG_CONFIG_HOME/imp/keys`, one `NAME=value` per line, written `0600`; tests for parsing, replacement, and the mode
- [x] 1.2 Resolve `IMP_API_KEY`, the profile variable in the environment, then the keys file, then `api_key`; tests for the order and for a stored key

## 2. Startup

- [x] 2.1 Record a missing key instead of failing, keep line mode's error and add the terminal hint, and pass the variable to the chat; tests for both branches

## 3. Onboarding

- [ ] 3.1 Open the provider picker when a key is missing, apply a keyless profile at once, and quit on Esc; tests for the picker and the quit
- [ ] 3.2 Mask the key row, accept characters and Backspace, and check the key on Enter; tests for the mask and the editing
- [ ] 3.3 Save and apply on success, retry on a 4xx, save with a warning otherwise; tests for the three outcomes
