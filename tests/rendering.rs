use mdcat::{render, RenderOptions};
use textwrap::core::display_width;

fn plain(markdown: &str, width: usize) -> String {
    render(
        markdown,
        &RenderOptions {
            width,
            ..Default::default()
        },
    )
}

#[test]
fn nested_lists_preserve_parent_text_and_indentation() {
    assert_eq!(
        plain("- Parent\n  - Child\n    1. Grandchild\n- Sibling", 40),
        "• Parent\n  • Child\n    1. Grandchild\n• Sibling\n"
    );
}

#[test]
fn loose_items_keep_paragraphs_and_markers() {
    assert_eq!(
        plain(
            "- First paragraph.\n\n  Second paragraph.\n\n- Another item.",
            40
        ),
        "• First paragraph.\n\n  Second paragraph.\n\n• Another item.\n"
    );
}

#[test]
fn ordered_markers_align_across_digit_boundaries() {
    assert_eq!(
        plain("9. First words wrap onto a second line.\n10. Next", 24),
        " 9. First words wrap\n    onto a second line.\n10. Next\n"
    );
}

#[test]
fn quotes_keep_soft_breaks_hard_breaks_and_paragraphs() {
    assert_eq!(plain("> First line\n> continues.  \n> New line.\n>\n> Another paragraph.\n>\n>> Inner quote.", 40),
        "│ First line continues.\n│ New line.\n│\n│ Another paragraph.\n│\n│ │ Inner quote.\n");
}

#[test]
fn inline_elements_stay_inside_their_containers() {
    let input = "# [Title](https://example.com)\n\n> Quoted `code` and [link](https://example.com).\n\n| Cell |\n| --- |\n| [link](https://example.com) and `code` |";
    let output = plain(input, 80);
    assert!(output.starts_with("Title (https://example.com)\n"));
    assert!(output.contains("│ Quoted `code` and link (https://example.com)."));
    assert!(output.ends_with("link (https://example.com) and `code`\n"));
}

#[test]
fn hard_breaks_survive_and_soft_breaks_become_spaces() {
    assert_eq!(
        plain("First\nsecond  \nthird\\\nfourth", 80),
        "First second\nthird\nfourth\n"
    );
}

#[test]
fn tasks_and_nested_tasks_keep_their_labels() {
    assert_eq!(
        plain("- [x] Done\n  - [ ] Next\n- [ ] Later", 40),
        "• [x] Done\n  • [ ] Next\n• [ ] Later\n"
    );
}

#[test]
fn footnote_definitions_have_labels_and_continuation_indents() {
    assert_eq!(
        plain(
            "A note[^source].\n\n[^source]: First paragraph.\n\n    Second paragraph.",
            40
        ),
        "A note[source].\n\n[source] First paragraph.\n\n         Second paragraph.\n"
    );
}

#[test]
fn links_show_destinations_and_autolinks_are_not_duplicated() {
    assert_eq!(
        plain(
            "[site](https://example.com) <https://example.com> ![A view](view.png)",
            100
        ),
        "site (https://example.com) https://example.com [image: A view] (view.png)\n"
    );
}

#[test]
fn terminal_controls_are_removed_including_decoded_entities() {
    let output = plain("hello\x1b[31m\x07world &#27;[2J &#x9b;bad", 80);
    assert!(!output.chars().any(|c| c.is_control() && c != '\n'));
    assert!(output.contains("hello[31mworld"));
}

#[test]
fn raw_html_is_omitted_but_inline_text_survives() {
    assert_eq!(
        plain("Hello <b>world</b>.\n\n<div>hidden</div>\n\nVisible.", 80),
        "Hello world.\n\nVisible.\n"
    );
}

#[test]
fn code_is_never_truncated_even_for_unknown_languages() {
    let long = "0123456789".repeat(20);
    for language in ["", "rust", "not-a-language"] {
        let output = plain(&format!("```{language}\n{long}\n```"), 20);
        assert!(output.contains(&long), "lost code for {language}");
    }
}

#[test]
fn indented_code_and_blank_lines_are_preserved() {
    assert_eq!(
        plain("    first\n\n    second", 40),
        "│ first\n│\n│ second\n"
    );
    assert_eq!(plain("```\n\tlet x = 1;\n```", 40), "│     let x = 1;\n");
}

#[test]
fn nested_code_retains_trailing_spaces() {
    assert_eq!(
        plain("> ```\n> significant spaces  \n> ```", 40),
        "│ │ significant spaces  \n"
    );
    assert_eq!(
        plain("- item\n\n  ```\n  significant spaces  \n  ```", 40),
        "• item\n\n  │ significant spaces  \n"
    );
}

#[test]
fn prose_wraps_in_display_columns_and_does_not_split_urls() {
    let output = plain("你好 世界 café café fin", 10);
    assert_eq!(output, "你好 世界\ncafé café\nfin\n");
    assert!(output.lines().all(|line| display_width(line) <= 10));
    assert_eq!(
        plain("https://example.com/long-path", 5),
        "https://example.com/long-path\n"
    );
}

#[test]
fn tables_align_unicode_and_respect_column_alignment() {
    assert_eq!(plain("| Name | Count | State |\n| :--- | ---: | :---: |\n| 東京 | 2 | ok |\n| café | 100 | yes |", 80),
        "Name │ Count │ State\n─────┼───────┼──────\n東京 │     2 │  ok\ncafé │   100 │  yes\n");
}

#[test]
fn narrow_tables_wrap_cells_and_keep_all_words() {
    let output = plain(
        "| Task | Notes |\n| --- | --- |\n| first task | several short words here |",
        22,
    );
    assert!(
        output.lines().all(|line| display_width(line) <= 22),
        "{output}"
    );
    for word in ["first", "task", "several", "short", "words", "here"] {
        assert!(output.contains(word), "missing {word}: {output}");
    }
}

#[test]
fn long_table_values_keep_separators_aligned() {
    let output = plain(
        "| Value | Next |\n| --- | --- |\n| supercalifragilistic | a |\n| tiny | b |",
        16,
    );
    let separators: Vec<_> = output
        .lines()
        .filter_map(|line| line.split_once('│').map(|(left, _)| display_width(left)))
        .collect();
    assert!(
        separators.iter().all(|width| *width == separators[0]),
        "{output}"
    );
}

#[test]
fn colored_output_has_styles_highlighting_and_balanced_hyperlinks() {
    let options = RenderOptions {
        width: 40,
        color: true,
        hyperlinks: true,
    };
    let output = render("**bold _nested_ after** [link](https://example.com)\n\n```rust extra-info\nfn main() {}\n```", &options);
    assert!(output.contains("\x1b[1m"));
    assert!(output.contains("\x1b[3m"));
    assert!(output.contains("\x1b[0m\x1b[1m after"));
    assert!(output.contains("\x1b[38;2;"));
    assert_eq!(
        output.matches("\x1b]8;;https://example.com\x1b\\").count(),
        1
    );
    assert_eq!(output.matches("\x1b]8;;\x1b\\").count(), 1);
    assert!(output.contains("(https://example.com)"));
    assert!(!output.contains("extra-info"));
}

#[test]
fn hyperlinks_can_be_disabled_independently_and_unsafe_schemes_are_inert() {
    let options = RenderOptions {
        color: true,
        ..Default::default()
    };
    assert!(!render("[link](https://example.com)", &options).contains("\x1b]8"));
    let options = RenderOptions {
        hyperlinks: true,
        ..options
    };
    let output = render("[link](javascript:alert%281%29)", &options);
    assert!(!output.contains("\x1b]8"));
}

#[test]
fn ansi_and_osc_do_not_change_wrapping() {
    let input = "Some **bold words** then [a link](https://example.com) then more words.";
    let plain_output = plain(input, 24);
    let styled = render(
        input,
        &RenderOptions {
            width: 24,
            color: true,
            hyperlinks: true,
        },
    );
    let widths = |text: &str| text.lines().map(display_width).collect::<Vec<_>>();
    assert_eq!(widths(&plain_output), widths(&styled));
}

#[test]
fn empty_input_and_spacing_have_no_surprise_blank_lines() {
    for input in ["", "\n\n", "<!-- comment -->"] {
        assert_eq!(plain(input, 80), "");
    }
    assert_eq!(plain("\n\nText.\n\n\n", 80), "Text.\n");
    assert_eq!(plain("## Title", 80), "Title\n─────\n");
    assert_eq!(plain("Text.\r\n\r\nNext.", 80), "Text.\n\nNext.\n");
}

#[test]
fn tiny_widths_and_incomplete_markdown_do_not_panic() {
    for width in [0, 1, 2, 10, 1001, usize::MAX] {
        for input in [
            "# Heading",
            "> > - item",
            "| a | b |\n| - | - |\n| c | d |",
            "```rust\nfn main() {",
            "**unfinished",
            "-",
            "![]()",
        ] {
            let output = plain(input, width);
            assert!(!output.contains('\x1b'));
        }
    }
}

#[test]
fn example_snapshots() {
    for (name, input, expected) in [
        (
            "showcase",
            include_str!("../examples/showcase.md"),
            include_str!("snapshots/showcase.txt"),
        ),
        (
            "nested",
            include_str!("../examples/nested.md"),
            include_str!("snapshots/nested.txt"),
        ),
        (
            "tables",
            include_str!("../examples/tables.md"),
            include_str!("snapshots/tables.txt"),
        ),
        (
            "code",
            include_str!("../examples/code.md"),
            include_str!("snapshots/code.txt"),
        ),
        (
            "edge-cases",
            include_str!("../examples/edge-cases.md"),
            include_str!("snapshots/edge-cases.txt"),
        ),
    ] {
        assert_eq!(plain(input, 60), expected, "example: {name}");
    }
}
