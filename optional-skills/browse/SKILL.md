---
name: browse
description: Read a web page's text, or render a JavaScript-heavy page that fetch returns empty. Use when asked to open, read, check, or scrape a URL. Uses fetch first and obscura only when a page needs JavaScript.
---

# Browse a page

## Order

1. Use `fetch` for the URL. It returns HTML as markdown, and that covers most pages.
2. Only if `fetch` returns an empty shell, or the text says the page needs JavaScript,
   render it with obscura.

## Check obscura first

```sh
command -v obscura || echo "obscura not installed"
```

If it is missing, report the install hint and stop. Do not install it yourself.
Install from the official release: https://github.com/h4ckf0r0day/obscura/releases
(macOS Apple Silicon: `obscura-aarch64-macos.tar.gz`).

## Render with obscura

```sh
obscura fetch "<url>" --eval "document.title"
obscura fetch "<url>" --eval "document.body.innerText"
```

On macOS, obscura does not use the system keychain, so a certificate check can fail with
`UnknownIssuer`. Give it the system roots as a PEM file and keep verification on:

```sh
security find-certificate -a -p /System/Library/Keychains/SystemRootCertificates.keychain \
  > ~/.local/state/imp/system-roots.pem
SSL_CERT_FILE=~/.local/state/imp/system-roots.pem obscura fetch "<url>" --eval "document.body.innerText"
```

Do not disable certificate checks.

## Limits

- Use the default settings. Do not enable `--stealth` or any bot-protection option, and do
  not use them against a site whose terms forbid automated access.
- Pages on private addresses are blocked unless the user asks; `--allow-private-network`
  needs their approval.
- Output can be long. Summarise it, or pipe through `head`.
