use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};
use std::sync::LazyLock;
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};
use textwrap::{core::display_width, WordSeparator, WordSplitter, WrapAlgorithm};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Output policy. The library defaults to plain text at 80 columns.
#[derive(Clone, Debug)]
pub struct RenderOptions {
    /// Prose width in terminal columns, clamped to 1..=1000.
    /// Code, long unbroken words, and very narrow tables may exceed this width.
    pub width: usize,
    /// Emit ANSI styles and syntax colors.
    pub color: bool,
    /// Make HTTP(S) and mailto links clickable with OSC 8 escapes.
    /// Link destinations remain visible whether this is enabled or disabled.
    pub hyperlinks: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 80,
            color: false,
            hyperlinks: false,
        }
    }
}

/// Render Markdown with tables, task lists, strikethrough, and footnotes.
///
/// Raw HTML is omitted; images become descriptive placeholders. Terminal control
/// characters in the input are removed. Nonempty output ends in one newline.
pub fn render(markdown: &str, options: &RenderOptions) -> String {
    let cleaned = clean(markdown);
    let parser = Parser::new_ext(
        &cleaned,
        Options::ENABLE_TABLES
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_FOOTNOTES,
    );
    // Each container owns its children: paragraphs in quotes, lists, and table
    // cells cannot overwrite one another's buffers.
    let nodes = parse(&mut parser.into_iter());
    let renderer = Renderer { options };
    let output = renderer.blocks(&nodes, options.width.clamp(1, 1000), false);
    if output.is_empty() {
        output
    } else {
        format!("{}\n", output.trim_end_matches('\n'))
    }
}

enum Node<'a> {
    Element(Tag<'a>, Vec<Node<'a>>),
    Leaf(Event<'a>),
}

fn parse<'a>(events: &mut impl Iterator<Item = Event<'a>>) -> Vec<Node<'a>> {
    let mut nodes = Vec::new();
    while let Some(event) = events.next() {
        match event {
            Event::Start(tag) => nodes.push(Node::Element(tag, parse(events))),
            Event::End(_) => break,
            event => nodes.push(Node::Leaf(event)),
        }
    }
    nodes
}

fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t' | '\r'))
        .collect()
}

fn is_block(node: &Node<'_>) -> bool {
    match node {
        Node::Element(tag, _) => !matches!(
            tag,
            Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
        ),
        Node::Leaf(Event::Rule) => true,
        _ => false,
    }
}

fn wrap(text: &str, width: usize) -> String {
    textwrap::fill(
        text,
        textwrap::Options::new(width.max(1))
            .break_words(false)
            .word_separator(WordSeparator::AsciiSpace)
            .word_splitter(WordSplitter::NoHyphenation)
            .wrap_algorithm(WrapAlgorithm::FirstFit),
    )
}

fn prefix(text: &str, first: &str, rest: &str) -> String {
    text.split('\n')
        .enumerate()
        .map(|(i, line)| {
            let gutter = if i == 0 { first } else { rest };
            if line.is_empty() {
                gutter.trim_end().to_string()
            } else {
                format!("{gutter}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

struct Renderer<'a> {
    options: &'a RenderOptions,
}

impl Renderer<'_> {
    fn style(&self, text: &str, codes: &str) -> String {
        if self.options.color && !text.is_empty() {
            // Restore the parent's style after a nested inline span resets it.
            let start = format!("\x1b[{codes}m");
            format!(
                "{start}{}\x1b[0m",
                text.replace("\x1b[0m", &format!("\x1b[0m{start}"))
            )
        } else {
            text.to_string()
        }
    }

    fn inline(&self, nodes: &[Node<'_>]) -> String {
        let mut out = String::new();
        for node in nodes {
            let text = match node {
                Node::Leaf(Event::Text(text)) => clean(text),
                Node::Leaf(Event::Code(text)) => self.style(&format!("`{}`", clean(text)), "33"),
                Node::Leaf(Event::SoftBreak) => " ".into(),
                Node::Leaf(Event::HardBreak) => "\n".into(),
                Node::Leaf(Event::TaskListMarker(checked)) => self.style(
                    if *checked { "[x] " } else { "[ ] " },
                    if *checked { "32" } else { "2" },
                ),
                Node::Leaf(Event::FootnoteReference(label)) => {
                    self.style(&format!("[{label}]"), "36")
                }
                Node::Element(tag, children) => {
                    let label = self.inline(children);
                    match tag {
                        Tag::Emphasis => self.style(&label, "3"),
                        Tag::Strong => self.style(&label, "1"),
                        Tag::Strikethrough => self.style(&label, "9"),
                        Tag::Link { dest_url, .. } => {
                            let url = clean(dest_url);
                            let label = self.style(&label, "4;34");
                            let label = if self.options.hyperlinks
                                && ["https://", "http://", "mailto:"]
                                    .iter()
                                    .any(|scheme| url.starts_with(scheme))
                            {
                                format!("\x1b]8;;{url}\x1b\\{label}\x1b]8;;\x1b\\")
                            } else {
                                label
                            };
                            if self.inline_plain(children) == url {
                                label
                            } else {
                                format!("{label} ({url})")
                            }
                        }
                        Tag::Image { dest_url, .. } => {
                            let alt = if label.is_empty() { "image" } else { &label };
                            format!("[image: {alt}] ({})", clean(dest_url))
                        }
                        _ => label,
                    }
                }
                _ => String::new(),
            };
            out.push_str(&text);
        }
        out
    }

    fn inline_plain(&self, nodes: &[Node<'_>]) -> String {
        Renderer {
            options: &RenderOptions::default(),
        }
        .inline(nodes)
    }

    fn blocks(&self, nodes: &[Node<'_>], width: usize, tight: bool) -> String {
        let mut chunks = Vec::new();
        let mut start = 0;
        for (index, node) in nodes.iter().enumerate() {
            if !is_block(node) {
                continue;
            }
            if start < index {
                chunks.push(wrap(&self.inline(&nodes[start..index]), width));
            }
            chunks.push(self.block(node, width));
            start = index + 1;
        }
        if start < nodes.len() {
            chunks.push(wrap(&self.inline(&nodes[start..]), width));
        }
        chunks.retain(|chunk| !chunk.is_empty());
        chunks.join(if tight { "\n" } else { "\n\n" })
    }

    fn block(&self, node: &Node<'_>, width: usize) -> String {
        let Node::Element(tag, children) = node else {
            return self.style(&"─".repeat(width), "2");
        };
        match tag {
            Tag::Paragraph => wrap(&self.inline(children), width),
            Tag::Heading { level, .. } => {
                let text = wrap(&self.inline(children), width);
                let heading = self.style(&text, "1");
                match level {
                    HeadingLevel::H1 => {
                        format!("{heading}\n{}", self.style(&"━".repeat(width), "2"))
                    }
                    HeadingLevel::H2 => {
                        let length = text
                            .lines()
                            .map(display_width)
                            .max()
                            .unwrap_or(0)
                            .min(width);
                        format!("{heading}\n{}", self.style(&"─".repeat(length), "2"))
                    }
                    _ => heading,
                }
            }
            Tag::BlockQuote => {
                let text = self.blocks(children, width.saturating_sub(2).max(1), false);
                let gutter = format!("{} ", self.style("│", "2"));
                prefix(&text, &gutter, &gutter)
            }
            Tag::List(first) => {
                let loose = children.iter().any(|item| matches!(item,
                    Node::Element(Tag::Item, contents) if contents.iter().any(|n| matches!(n, Node::Element(Tag::Paragraph, _)))));
                let mut items = Vec::new();
                let mut number = first.unwrap_or(1);
                let marker_width = first
                    .map(|n| {
                        n.saturating_add(children.len().saturating_sub(1) as u64)
                            .to_string()
                            .len()
                            + 2
                    })
                    .unwrap_or(2);
                for item in children {
                    if let Node::Element(Tag::Item, contents) = item {
                        let marker = if first.is_some() {
                            let marker = format!("{number}.");
                            format!("{marker:>padding$} ", padding = marker_width - 1)
                        } else {
                            "• ".to_string()
                        };
                        let text = self.blocks(
                            contents,
                            width.saturating_sub(marker_width).max(1),
                            !loose,
                        );
                        items.push(prefix(
                            &text,
                            &self.style(&marker, "2"),
                            &" ".repeat(marker_width),
                        ));
                        number = number.saturating_add(1);
                    }
                }
                items.join(if loose { "\n\n" } else { "\n" })
            }
            Tag::CodeBlock(kind) => {
                let code = children
                    .iter()
                    .filter_map(|node| match node {
                        Node::Leaf(Event::Text(text)) => Some(text.as_ref()),
                        _ => None,
                    })
                    .collect::<String>();
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or(""),
                    CodeBlockKind::Indented => "",
                };
                self.code(&code, lang)
            }
            Tag::Table(alignments) => self.table(children, alignments, width),
            Tag::FootnoteDefinition(label) => {
                let marker = format!("[{label}] ");
                let indent = " ".repeat(display_width(&marker));
                let text = self.blocks(children, width.saturating_sub(indent.len()).max(1), false);
                prefix(&text, &self.style(&marker, "36"), &indent)
            }
            Tag::HtmlBlock => String::new(),
            _ => self.blocks(children, width, false),
        }
    }

    fn code(&self, code: &str, lang: &str) -> String {
        let gutter = self.style("│", "2");
        let mut lines = Vec::new();
        if !lang.is_empty() {
            lines.push(format!("{gutter} {}", self.style(lang, "2")));
        }
        // Only initialize syntax definitions when color highlighting is needed,
        // and reuse them across all code blocks and render calls.
        let syntax = if self.options.color && !lang.is_empty() {
            SYNTAXES.find_syntax_by_token(lang)
        } else {
            None
        };
        let mut highlighter =
            syntax.map(|syntax| HighlightLines::new(syntax, &THEMES.themes["base16-ocean.dark"]));
        for line in code.split_inclusive('\n') {
            let expanded = expand_tabs(line);
            let highlighted = if let Some(highlighter) = &mut highlighter {
                match highlighter.highlight_line(&expanded, &SYNTAXES) {
                    Ok(spans) => spans
                        .into_iter()
                        .map(|(style, text)| {
                            let c = style.foreground;
                            self.style(
                                text.trim_end_matches('\n'),
                                &format!("38;2;{};{};{}", c.r, c.g, c.b),
                            )
                        })
                        .collect::<String>(),
                    Err(_) => expanded.trim_end_matches('\n').to_string(),
                }
            } else {
                expanded.trim_end_matches('\n').to_string()
            };
            lines.push(if highlighted.is_empty() {
                gutter.clone()
            } else {
                format!("{gutter} {highlighted}")
            });
        }
        if lines.is_empty() {
            gutter
        } else {
            lines.join("\n")
        }
    }

    fn table(&self, children: &[Node<'_>], alignments: &[Alignment], width: usize) -> String {
        let rows: Vec<Vec<String>> = children
            .iter()
            .filter_map(|row| match row {
                Node::Element(Tag::TableHead | Tag::TableRow, cells) => Some(
                    cells
                        .iter()
                        .filter_map(|cell| match cell {
                            Node::Element(Tag::TableCell, contents) => Some(self.inline(contents)),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .collect();
        let columns = alignments.len();
        if columns == 0 || rows.is_empty() {
            return String::new();
        }
        let mut widths = (0..columns)
            .map(|col| {
                rows.iter()
                    .map(|row| row.get(col).map(|s| display_width(s)).unwrap_or(0))
                    .max()
                    .unwrap_or(1)
                    .max(1)
            })
            .collect::<Vec<_>>();
        let available = width.saturating_sub((columns - 1) * 3).max(columns);
        while widths.iter().sum::<usize>() > available {
            let (i, widest) = widths.iter().enumerate().max_by_key(|(_, w)| **w).unwrap();
            if *widest <= 1 {
                break;
            }
            widths[i] -= 1;
        }
        // Unbroken values can exceed their assigned width. Expand that column
        // for every row so its separators still line up across the table.
        for (col, column_width) in widths.iter_mut().enumerate() {
            *column_width = (*column_width).max(
                rows.iter()
                    .map(|row| {
                        wrap(
                            row.get(col).map(String::as_str).unwrap_or(""),
                            *column_width,
                        )
                        .lines()
                        .map(display_width)
                        .max()
                        .unwrap_or(0)
                    })
                    .max()
                    .unwrap_or(0),
            );
        }
        let mut output = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let cells: Vec<Vec<String>> = widths
                .iter()
                .enumerate()
                .map(|(col, width)| {
                    wrap(row.get(col).map(String::as_str).unwrap_or(""), *width)
                        .split('\n')
                        .map(str::to_string)
                        .collect()
                })
                .collect();
            let height = cells.iter().map(Vec::len).max().unwrap_or(1);
            for line in 0..height {
                let parts = (0..columns)
                    .map(|col| {
                        let text = cells[col].get(line).map(String::as_str).unwrap_or("");
                        let padding = widths[col].saturating_sub(display_width(text));
                        let (left, right) = match alignments[col] {
                            Alignment::Right => (padding, 0),
                            Alignment::Center => (padding / 2, padding - padding / 2),
                            _ => (0, padding),
                        };
                        let text = if index == 0 {
                            self.style(text, "1")
                        } else {
                            text.to_string()
                        };
                        format!("{}{text}{}", " ".repeat(left), " ".repeat(right))
                    })
                    .collect::<Vec<_>>();
                output.push(parts.join(" │ ").trim_end().to_string());
            }
            if index == 0 {
                output.push(
                    self.style(
                        &widths
                            .iter()
                            .map(|w| "─".repeat(*w))
                            .collect::<Vec<_>>()
                            .join("─┼─"),
                        "2",
                    ),
                );
            }
        }
        output.join("\n")
    }
}

fn expand_tabs(text: &str) -> String {
    let mut result = String::new();
    let mut column = 0;
    for c in text.chars() {
        if c == '\t' {
            let spaces = 4 - column % 4;
            result.push_str(&" ".repeat(spaces));
            column += spaces;
        } else {
            result.push(c);
            column = if c == '\n' {
                0
            } else {
                column + display_width(&c.to_string())
            };
        }
    }
    result
}
