---
name: image
description: Show an image in the terminal, such as a picture, screenshot, or photo file. Use when asked to display, preview, or show an image. Renders it with chafa.
---

# Show an image

## Check chafa first

```sh
command -v chafa || echo "chafa not installed"
```

If it is missing, report the install hint and stop. Do not install it yourself:
`brew install chafa`.

## Render

Run chafa with the symbol format, which works in any terminal, and show the output:

```sh
chafa --format symbols --size 80x40 "<path>"
```

Call `bash` with `show: true` so the user sees the colours. You get only the text, or the
size of the picture.

## Limits

- Only render files the user named or that are in the session directory.
- If the file is not an image chafa supports, report the error and do not convert it.
