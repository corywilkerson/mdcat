# Example gallery

From the project directory, run an example with:

```sh
cargo run --locked -- examples/showcase.md
cargo run --locked -- --width 40 examples/nested.md
cargo run --locked -- --color always examples/code.md
cargo run --locked -- --plain --width 60 examples/tables.md
```

| Example | What to look for |
| --- | --- |
| `showcase.md` | Headings H1–H6, inline styles, lists, links, quotes, rules, images, footnotes |
| `nested.md` | Parent items, nested lists, loose items, quotes, numbering from 9 to 10 |
| `tables.md` | Left/right/center alignment, Unicode widths, wrapping, empty cells, long values |
| `code.md` | Several languages, unknown syntax, blank lines, long lines, quoted code |
| `edge-cases.md` | Hard breaks, nested styles, references, HTML policy, incomplete Markdown |

`tests/snapshots/` contains the reviewed plain-text output at 60 columns.
`cargo test --locked` checks each example against its snapshot. For an intentional
formatting change, regenerate the previews with
`python3 scripts/update-snapshots.py`, inspect the diff, and rerun the tests.

Code, unbroken words, and tables whose values cannot fit may exceed the requested
width. They are preserved rather than truncated.
