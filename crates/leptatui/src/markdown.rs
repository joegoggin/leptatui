//! CommonMark parsing and semantic document-view conversion.
//!
//! This module converts pulldown-cmark's balanced event stream into the
//! semantic headings, paragraphs, lists, and tables exposed by [`crate::view`].
//! Only the table extension is enabled beyond core CommonMark.

use std::iter::Peekable;

use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::{
    CellAlignment, View, column, h1, h2, h3, h4, h5, h6, list_item, ordered_list, paragraph, table,
    table_body, table_cell, table_head, table_row, unordered_list,
};

/// Converts CommonMark source into a scrollable semantic document view.
///
/// The parser enables tables as the only extension. Unsupported block types
/// are traversed when they can contain supported semantic blocks so their
/// readable child content remains in source order.
///
/// # Arguments
///
/// * `source` — CommonMark source text to parse.
///
/// # Returns
///
/// A [`View::Column`] containing semantic document blocks in source order.
pub(crate) fn markdown_to_view(source: &str) -> View {
    let mut parser = Parser::new_ext(source, Options::ENABLE_TABLES).peekable();
    column(parse_blocks(&mut parser, None))
}

/// Parses semantic blocks until the requested closing tag or end of input.
///
/// Direct inline events are collected into a paragraph because pulldown-cmark
/// omits paragraph tags inside tight list items.
///
/// # Arguments
///
/// * `events` — Peekable CommonMark event stream positioned inside a block.
/// * `end` — Optional closing tag that terminates the current block sequence.
///
/// # Returns
///
/// A [`Vec`] containing converted semantic views in event order.
fn parse_blocks<'a>(events: &mut Peekable<Parser<'a>>, end: Option<TagEnd>) -> Vec<View> {
    let mut views = Vec::new();
    let mut inline = String::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(paragraph(parse_inline(events, TagEnd::Paragraph)));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                flush_inline_paragraph(&mut inline, &mut views);
                let content = parse_inline(events, TagEnd::Heading(level));
                views.push(heading(level, content));
            }
            Event::Start(Tag::List(start)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_list(events, start));
            }
            Event::Start(Tag::Table(alignments)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_table(events, &alignments));
            }
            Event::Start(Tag::BlockQuote(kind)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.extend(parse_blocks(events, Some(TagEnd::BlockQuote(kind))));
            }
            Event::Start(Tag::CodeBlock(_)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                skip_until(events, TagEnd::CodeBlock);
            }
            Event::Start(Tag::HtmlBlock) => {
                flush_inline_paragraph(&mut inline, &mut views);
                skip_until(events, TagEnd::HtmlBlock);
            }
            Event::Text(content) | Event::Code(content) => inline.push_str(&content),
            Event::SoftBreak | Event::HardBreak => inline.push('\n'),
            Event::End(tag) if Some(tag) == end => {
                flush_inline_paragraph(&mut inline, &mut views);
                break;
            }
            Event::End(_)
            | Event::Start(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_)
            | Event::Rule
            | Event::TaskListMarker(_) => {}
        }
    }

    flush_inline_paragraph(&mut inline, &mut views);
    views
}

/// Converts accumulated direct inline content into a semantic paragraph.
///
/// # Arguments
///
/// * `inline` — Pending unstyled inline text.
/// * `views` — Destination block sequence for the resulting paragraph.
fn flush_inline_paragraph(inline: &mut String, views: &mut Vec<View>) {
    if !inline.is_empty() {
        views.push(paragraph(std::mem::take(inline)));
    }
}

/// Collects unstyled inline content through a matching closing tag.
///
/// Inline styling and terminal-specific link rendering are layered onto this
/// conversion in later Markdown work. Soft and hard breaks both remain
/// explicit text line boundaries.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned after an opening tag.
/// * `end` — Closing tag that terminates the inline content.
///
/// # Returns
///
/// An owned [`String`] containing the inline text and retained line breaks.
fn parse_inline<'a>(events: &mut Peekable<Parser<'a>>, end: TagEnd) -> String {
    let mut content = String::new();

    while let Some(event) = events.next() {
        match event {
            Event::Text(text) | Event::Code(text) => content.push_str(&text),
            Event::SoftBreak | Event::HardBreak => content.push('\n'),
            Event::End(tag) if tag == end => break,
            Event::Start(Tag::CodeBlock(_)) => skip_until(events, TagEnd::CodeBlock),
            Event::Start(Tag::HtmlBlock) => skip_until(events, TagEnd::HtmlBlock),
            Event::Start(_)
            | Event::End(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_)
            | Event::Rule
            | Event::TaskListMarker(_) => {}
        }
    }

    content
}

/// Creates the semantic heading matching a CommonMark heading level.
///
/// # Arguments
///
/// * `level` — Parsed CommonMark heading level.
/// * `content` — Owned unstyled heading content.
///
/// # Returns
///
/// A semantic H1 through H6 [`View`].
fn heading(level: HeadingLevel, content: String) -> View {
    match level {
        HeadingLevel::H1 => h1(content),
        HeadingLevel::H2 => h2(content),
        HeadingLevel::H3 => h3(content),
        HeadingLevel::H4 => h4(content),
        HeadingLevel::H5 => h5(content),
        HeadingLevel::H6 => h6(content),
    }
}

/// Parses a CommonMark ordered or unordered list.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned after the list opening tag.
/// * `start` — First ordered marker, or [`None`] for an unordered list.
///
/// # Returns
///
/// A semantic ordered or unordered list retaining item order and nesting.
fn parse_list<'a>(events: &mut Peekable<Parser<'a>>, start: Option<u64>) -> View {
    let mut items = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::Item) => {
                items.push(list_item(parse_blocks(events, Some(TagEnd::Item))));
            }
            Event::End(TagEnd::List(_)) => break,
            _ => {}
        }
    }

    match start {
        Some(start) => {
            let start = usize::try_from(start).unwrap_or(usize::MAX);
            ordered_list(items).start(start)
        }
        None => unordered_list(items),
    }
}

/// Parses a CommonMark table into semantic header and body sections.
///
/// Pulldown-cmark emits header cells directly inside `TableHead`, so this
/// conversion creates the semantic header row that Leptatui tables require.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned after the table opening tag.
/// * `alignments` — Parsed alignment for each source column.
///
/// # Returns
///
/// A semantic table containing one header section and one body section.
fn parse_table<'a>(events: &mut Peekable<Parser<'a>>, alignments: &[Alignment]) -> View {
    let mut header_rows = Vec::new();
    let mut body_rows = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::TableHead) => {
                header_rows.push(parse_table_head(events, alignments));
            }
            Event::Start(Tag::TableRow) => {
                body_rows.push(parse_table_row(events, alignments));
            }
            Event::End(TagEnd::Table) => break,
            _ => {}
        }
    }

    table([table_head(header_rows), table_body(body_rows)])
}

/// Parses direct CommonMark table-header cells into one semantic row.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside a table head.
/// * `alignments` — Parsed alignment for each source column.
///
/// # Returns
///
/// A semantic table-row view containing the header cells.
fn parse_table_head<'a>(events: &mut Peekable<Parser<'a>>, alignments: &[Alignment]) -> View {
    let mut cells = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::TableCell) => {
                let alignment = alignment_at(alignments, cells.len());
                cells
                    .push(table_cell(parse_inline(events, TagEnd::TableCell)).alignment(alignment));
            }
            Event::End(TagEnd::TableHead) => break,
            _ => {}
        }
    }

    table_row(cells)
}

/// Parses one CommonMark body row into a semantic table row.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside a table row.
/// * `alignments` — Parsed alignment for each source column.
///
/// # Returns
///
/// A semantic table-row view containing aligned body cells.
fn parse_table_row<'a>(events: &mut Peekable<Parser<'a>>, alignments: &[Alignment]) -> View {
    let mut cells = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::TableCell) => {
                let alignment = alignment_at(alignments, cells.len());
                cells
                    .push(table_cell(parse_inline(events, TagEnd::TableCell)).alignment(alignment));
            }
            Event::End(TagEnd::TableRow) => break,
            _ => {}
        }
    }

    table_row(cells)
}

/// Returns the semantic alignment for one parsed table column.
///
/// Missing and unspecified alignments use the semantic cell's left default.
///
/// # Arguments
///
/// * `alignments` — Parsed table-column alignments.
/// * `column` — Zero-based source column index.
///
/// # Returns
///
/// A [`CellAlignment`] for the requested table cell.
fn alignment_at(alignments: &[Alignment], column: usize) -> CellAlignment {
    match alignments.get(column).copied().unwrap_or(Alignment::None) {
        Alignment::None | Alignment::Left => CellAlignment::Left,
        Alignment::Center => CellAlignment::Center,
        Alignment::Right => CellAlignment::Right,
    }
}

/// Discards events through a balanced unsupported block's closing tag.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside the block.
/// * `end` — Closing tag that terminates the discarded block.
fn skip_until<'a>(events: &mut Peekable<Parser<'a>>, end: TagEnd) {
    for event in events.by_ref() {
        if event == Event::End(end) {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies Markdown headings map to every semantic heading level.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// # One
    /// ## Two
    /// ### Three
    /// #### Four
    /// ##### Five
    /// ###### Six
    /// ```
    ///
    /// # Assertions
    ///
    /// - Parsing succeeds without a fallible API.
    /// - H1 through H6 appear in source order.
    /// - Every heading retains its text content.
    #[test]
    fn markdown_maps_all_heading_levels() {
        let source = concat!(
            "# One\n\n",
            "## Two\n\n",
            "### Three\n\n",
            "#### Four\n\n",
            "##### Five\n\n",
            "###### Six\n",
        );

        assert_eq!(
            markdown_to_view(source),
            column([
                h1("One"),
                h2("Two"),
                h3("Three"),
                h4("Four"),
                h5("Five"),
                h6("Six"),
            ]),
        );
    }

    /// Verifies Markdown paragraphs retain both line-break forms and Unicode.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Soft
    /// break\
    /// hard 界 `code`
    /// ```
    ///
    /// # Assertions
    ///
    /// - Parsing succeeds without a fallible API.
    /// - Soft and hard breaks become explicit text line boundaries.
    /// - Unicode and inline-code content remain intact.
    #[test]
    fn markdown_preserves_paragraph_breaks_and_unicode() {
        let source = "Soft\nbreak  \nhard 界 `code`\n";

        assert_eq!(
            markdown_to_view(source),
            column([paragraph("Soft\nbreak\nhard 界 code")]),
        );
    }

    /// Verifies Markdown lists retain starts, blocks, nesting, and empty items.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// 3. First
    ///
    ///    Second paragraph.
    ///
    ///    - Nested bullet
    ///
    ///      7. Nested number
    ///    -
    /// 4. Last
    /// ```
    ///
    /// # Assertions
    ///
    /// - The outer ordered list starts at three.
    /// - Loose item paragraphs remain separate blocks.
    /// - Mixed ordered and unordered nesting retains its hierarchy.
    /// - Tight-list text becomes paragraphs and empty items remain present.
    #[test]
    fn markdown_preserves_nested_and_mixed_lists() {
        let source = concat!(
            "3. First\n\n",
            "   Second paragraph.\n\n",
            "   - Nested bullet\n\n",
            "     7. Nested number\n",
            "   -\n",
            "4. Last\n",
        );

        assert_eq!(
            markdown_to_view(source),
            column([ordered_list([
                list_item([
                    paragraph("First"),
                    paragraph("Second paragraph."),
                    unordered_list([
                        list_item([
                            paragraph("Nested bullet"),
                            ordered_list([list_item([paragraph("Nested number")])]).start(7),
                        ]),
                        list_item([]),
                    ]),
                ]),
                list_item([paragraph("Last")]),
            ])
            .start(3)]),
        );
    }

    /// Verifies Markdown tables map sections, rows, cells, and alignments.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// | Default | Left | Center | Right |
    /// | ------- | :--- | :----: | ----: |
    /// | a       | b    | c      | d     |
    /// ```
    ///
    /// # Assertions
    ///
    /// - Parsing succeeds with the table extension enabled.
    /// - Header cells are wrapped in a synthesized semantic header row.
    /// - Body rows remain in the semantic table body.
    /// - Default, left, center, and right alignments map to semantic cells.
    #[test]
    fn markdown_maps_table_structure_and_alignment() {
        let source = concat!(
            "| Default | Left | Center | Right |\n",
            "| ------- | :--- | :----: | ----: |\n",
            "| a       | b    | c      | d     |\n",
        );
        let aligned_cells = |values: [&'static str; 4]| {
            [
                table_cell(values[0]).alignment(CellAlignment::Left),
                table_cell(values[1]).alignment(CellAlignment::Left),
                table_cell(values[2]).alignment(CellAlignment::Center),
                table_cell(values[3]).alignment(CellAlignment::Right),
            ]
        };

        assert_eq!(
            markdown_to_view(source),
            column([table([
                table_head([table_row(aligned_cells([
                    "Default", "Left", "Center", "Right",
                ]))]),
                table_body([table_row(aligned_cells(["a", "b", "c", "d"]))]),
            ])]),
        );
    }

    /// Verifies Markdown conversion preserves mixed block source order.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// # 開始
    ///
    /// Before.
    ///
    /// - 中
    ///
    /// ## 終了
    /// ```
    ///
    /// # Assertions
    ///
    /// - Parsing succeeds without reordering block types.
    /// - Unicode content remains intact in headings and list items.
    #[test]
    fn markdown_preserves_source_order() {
        let source = "# 開始\n\nBefore.\n\n- 中\n\n## 終了\n";

        assert_eq!(
            markdown_to_view(source),
            column([
                h1("開始"),
                paragraph("Before."),
                unordered_list([list_item([paragraph("中")])]),
                h2("終了"),
            ]),
        );
    }

    /// Verifies empty Markdown produces an empty scrollable document.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// source = ""
    /// ```
    ///
    /// # Assertions
    ///
    /// - Parsing succeeds without a fallible API.
    /// - The result is an empty semantic column.
    #[test]
    fn markdown_empty_source_returns_empty_column() {
        assert_eq!(markdown_to_view(""), column([]));
    }
}
