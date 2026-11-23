# Design

## Approaches

- **Flat bar on the select background (chosen).** Same in every theme, mono
  included; one colour per row on a software-rendered console.
- **A gradient.** Bands in 256 colours, vanishes without colour, and changes
  colour in every cell.

## Details

`layout` reserves the top row when the screen has eight rows or more; body
targets and confirmation chips move down one, while overlays, placed from
the bottom, do not. The time comes from `localtime_r`, whose `tm` starts
with seconds, minutes and hours on macOS and glibc alike. The run loop
repaints when the minute shown differs from the clock.
