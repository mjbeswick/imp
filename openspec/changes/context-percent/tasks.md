# Tasks

## 1. Window size

- [x] 1.1 `context_window` config key parses a positive integer and rejects zero or non-numbers; tests in `config.rs`
- [x] 1.2 Window lookup returns the config value first, then the table entry, then nothing; tests for each source
- [x] 1.3 Table entries each cite their source in a comment; a test checks the table is non-empty and has no duplicate prefixes

## 2. Percent on the status line

- [ ] 2.1 `context_percent(tokens, window)` returns `min(100, tokens * 100 / window)` and nothing for an unknown window; tests for rounding down, the cap, and unknown
- [ ] 2.2 `status_line` shows `NN%` after the context tokens when known and omits it when not; tests for the scenarios "Context percent", "Percent rounds down", "Percent capped", "Window from config", and "Window unknown"
- [ ] 2.3 Status line stays within the width when the percent is added; test at 40 columns
