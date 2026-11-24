---
name: mermaid
description: Draw a Mermaid diagram in the terminal, such as a flowchart, state, or pie diagram. Use when asked to show, render, or preview a Mermaid diagram. Renders it as Unicode or ASCII text with graphs-tui.
---

# Draw a Mermaid diagram

## Check graphs-tui first

```sh
command -v graphs-tui || echo "graphs-tui not installed"
```

If it is missing, report the install hint and stop. Do not install it yourself:
`cargo install graphs-tui`.

## Render

Write the diagram source to a file in the session directory, then render it:

```sh
graphs-tui "<path-to-diagram.mmd>"
```

Call `bash` with `show: true` so the user sees the drawing. Supported here: flowcharts,
state diagrams, and pie charts. Other diagram kinds are not covered.

## Limits

- Render only the diagram the user gave you. Do not run code from a diagram.
- Bar and line charts are not supported. Say so instead of drawing a substitute.
