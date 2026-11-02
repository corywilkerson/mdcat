# Changelog

## 0.1.0

- Preserve parent items, nested lists, loose paragraphs, and nested quotes.
- Keep inline formatting, links, and code in their containing blocks and cells.
- Preserve hard breaks and display footnote definitions with labels.
- Wrap prose and table cells using terminal display widths and ANSI-aware layout.
- Honor table alignments and retain complete code lines and unbroken values.
- Cache syntax definitions; highlight known languages and show unknown ones plainly.
- Add width/color controls, plain output for pipes, visible URLs, and optional hyperlinks.
- Handle CLI errors, invalid UTF-8, non-UTF-8 file paths, and broken pipes cleanly.
- Add a library API, regression tests, reviewed example previews, CI, and packaging.
