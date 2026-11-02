# Small details

## Soft and hard breaks

This source line
joins the next one.  
This line starts after a hard break.\
So does this one.

## Inline nesting

**Bold with _emphasis_ and `code`, then bold again.**

[A **bold** label](https://example.com) and
<https://example.com> both retain their destinations.

## References

Read the [reference link][guide]. Here is a footnote.[^note]

[guide]: https://example.com/guide

[^note]: A note with **formatting** and `code`.

    A second paragraph stays attached to its label.

## Images and HTML

![Diagram of a small system](diagram.svg)

Inline <strong>HTML tags</strong> disappear; their text stays.

<div>This raw HTML block is omitted.</div>

The next paragraph remains visible.

## Unicode

你好 世界. Café, naïve, and é all belong in terminal text.

## Incomplete input

An unmatched **marker is readable, and an unclosed fence
still preserves its contents:

```unknown
unfinished, but not lost
