# Tables, without surprises

## Alignment and Unicode

| City | Items | Status |
| :--- | ---: | :---: |
| 東京 | 2 | ready |
| Zürich | 100 | queued |
| Montréal | 7 | done |

## Wrapping and inline content

| Task | Notes |
| :--- | :--- |
| Review **the patch** | Keep all the words when a cell wraps onto another line. |
| Run `cargo test` | A short command, followed by a longer explanation of the result. |
| Read [the guide](https://example.com) | Links stay inside their cells. |

## Empty cells

| Left | Middle | Right |
| --- | --- | --- |
| one | | three |
| | two | |

## Long values stay intact

| Identifier | Value |
| --- | ---: |
| a_really_long_unbroken_identifier | 42 |
| short | 9 |
