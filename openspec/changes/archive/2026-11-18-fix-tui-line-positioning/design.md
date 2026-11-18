# Design

The frame renderer owns terminal row boundaries. Replace bare line feeds with
CRLF for blank, content, input, and menu rows. This resets the terminal column
without changing cursor layout or row counts. Test the frame string by removing
all CRLF pairs and asserting no line feed remains.

No terminal emulator or dependency is needed. Existing frame tests cover the
rendered output; a focused test protects every row boundary.
