//! CommonMark parsing and semantic document-view conversion.
//!
//! This module converts pulldown-cmark's balanced event stream into the
//! semantic headings, paragraphs, lists, tables, highlighted code blocks, and
//! styled inline spans exposed by [`crate::view`]. Readable styled-block or
//! text fallbacks retain CommonMark content without dedicated semantic views.
//! Only the table extension is enabled beyond core CommonMark.

use std::iter::Peekable;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
};

use crate::{
    Borders, CellAlignment, TuiSpacing, TuiStyle, View, block, code_block, column, h1, h2, h3, h4,
    h5, h6, list_item, ordered_list, paragraph, table, table_body, table_cell, table_head,
    table_row, unordered_list,
};

/// Accumulates owned rich-text lines while parsing inline Markdown events.
///
/// Adjacent content with identical styles is coalesced so parser event
/// boundaries do not leak into the semantic view tree. An explicit content
/// flag distinguishes untouched tight-list content from a deliberately empty
/// line created by a Markdown break.
struct InlineText {
    /// Styled spans grouped by logical output line.
    lines: Vec<Vec<Span<'static>>>,
    /// Whether the parser emitted text or a line break into this accumulator.
    has_content: bool,
}

impl InlineText {
    /// Creates an empty rich-text accumulator with one logical line.
    ///
    /// # Returns
    ///
    /// An empty [`InlineText`] ready to receive styled content.
    fn new() -> Self {
        Self {
            lines: vec![Vec::new()],
            has_content: false,
        }
    }

    /// Returns whether any text or line break has been accumulated.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the accumulator contains parsed content.
    fn has_content(&self) -> bool {
        self.has_content
    }

    /// Appends styled text while preserving embedded logical line boundaries.
    ///
    /// Adjacent spans using the same style are merged on each line.
    ///
    /// # Arguments
    ///
    /// * `content` — Text emitted by the Markdown parser.
    /// * `style` — Ratatui style applied to the appended text.
    fn push_text(&mut self, content: &str, style: Style) {
        if content.is_empty() {
            return;
        }

        self.has_content = true;
        let mut parts = content.split('\n').peekable();
        while let Some(part) = parts.next() {
            if !part.is_empty() {
                self.push_span(part, style);
            }
            if parts.peek().is_some() {
                self.lines.push(Vec::new());
            }
        }
    }

    /// Appends one explicit logical line break.
    fn push_break(&mut self) {
        self.has_content = true;
        self.lines.push(Vec::new());
    }

    /// Appends another rich-text accumulator without losing its first line.
    ///
    /// # Arguments
    ///
    /// * `other` — Parsed inline content to append at the current position.
    fn append(&mut self, other: Self) {
        if !other.has_content {
            return;
        }

        self.has_content = true;
        for (index, line) in other.lines.into_iter().enumerate() {
            if index > 0 {
                self.lines.push(Vec::new());
            }
            for span in line {
                self.push_span(&span.content, span.style);
            }
        }
    }

    /// Returns the visible unstyled content represented by the accumulator.
    ///
    /// # Returns
    ///
    /// A [`String`] containing span content separated by logical newlines.
    fn plain_text(&self) -> String {
        self.lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Converts accumulated lines into owned Ratatui rich text.
    ///
    /// # Returns
    ///
    /// A [`Text`] containing every logical line and styled span.
    fn into_text(self) -> Text<'static> {
        Text::from(self.lines.into_iter().map(Line::from).collect::<Vec<_>>())
    }

    /// Appends one span or merges it with the matching previous span.
    ///
    /// # Arguments
    ///
    /// * `content` — Non-empty text for the span.
    /// * `style` — Ratatui style applied to the span.
    fn push_span(&mut self, content: &str, style: Style) {
        let line = self
            .lines
            .last_mut()
            .expect("inline text always retains one logical line");
        if let Some(span) = line.last_mut().filter(|span| span.style == style) {
            span.content.to_mut().push_str(content);
        } else {
            line.push(Span::styled(content.to_owned(), style));
        }
    }
}

impl Default for InlineText {
    /// Creates the default empty rich-text accumulator.
    ///
    /// # Returns
    ///
    /// An empty [`InlineText`] with one logical line.
    fn default() -> Self {
        Self::new()
    }
}

/// Converts CommonMark source into a scrollable semantic document view.
///
/// The parser enables tables as the only extension. Unsupported presentation
/// maps to deterministic terminal text or styled blocks, and unsupported
/// containers are traversed so readable child content remains in source order.
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
    let mut inline = InlineText::new();

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
                views.push(block_quote(parse_blocks(
                    events,
                    Some(TagEnd::BlockQuote(kind)),
                )));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_code_block(events, kind));
            }
            Event::Start(Tag::HtmlBlock) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_html_block(events));
            }
            Event::Start(Tag::Emphasis) => parse_inline_events(
                events,
                TagEnd::Emphasis,
                Style::new().add_modifier(Modifier::ITALIC),
                &mut inline,
            ),
            Event::Start(Tag::Strong) => parse_inline_events(
                events,
                TagEnd::Strong,
                Style::new().add_modifier(Modifier::BOLD),
                &mut inline,
            ),
            Event::Start(Tag::Link { dest_url, .. }) => {
                parse_link(events, &dest_url, Style::new(), &mut inline);
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                parse_image(events, &dest_url, Style::new(), &mut inline);
            }
            Event::Text(content)
            | Event::InlineMath(content)
            | Event::DisplayMath(content)
            | Event::Html(content)
            | Event::InlineHtml(content) => inline.push_text(&content, Style::new()),
            Event::Code(content) => {
                inline.push_text(&content, Style::new().add_modifier(Modifier::REVERSED))
            }
            Event::FootnoteReference(label) => {
                push_footnote_reference(&mut inline, &label, Style::new());
            }
            Event::SoftBreak | Event::HardBreak => inline.push_break(),
            Event::Rule => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(thematic_break());
            }
            Event::TaskListMarker(checked) => {
                push_task_list_marker(&mut inline, checked, Style::new());
            }
            Event::End(tag) if Some(tag) == end => {
                flush_inline_paragraph(&mut inline, &mut views);
                break;
            }
            Event::End(_) | Event::Start(_) => {}
        }
    }

    flush_inline_paragraph(&mut inline, &mut views);
    views
}

/// Wraps parsed quote children in a visible, wrap-aware terminal prefix.
///
/// A left border marks every rendered quote row, including wrapped content.
/// One cell of padding separates the marker from the content, and nested quote
/// blocks naturally stack their prefixes.
///
/// # Arguments
///
/// * `children` — Semantic and fallback views parsed inside the blockquote.
///
/// # Returns
///
/// A left-bordered [`View::Block`] containing the quote children.
fn block_quote(children: Vec<View>) -> View {
    block(column(children)).with_inline_style(
        TuiStyle::new()
            .borders(Borders::LEFT)
            .padding(TuiSpacing::new(1, 0, 0, 0)),
    )
}

/// Creates a width-responsive horizontal terminal rule.
///
/// # Returns
///
/// A one-row [`View::Block`] whose top border fills the available width.
fn thematic_break() -> View {
    block(column(Vec::<View>::new())).with_inline_style(TuiStyle::new().borders(Borders::TOP))
}

/// Collects one raw HTML block as literal terminal text.
///
/// Pulldown-cmark includes source line endings in block HTML events. Feeding
/// those payloads through [`InlineText`] retains the exact logical line shape
/// without interpreting tags or entities inside the raw block.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside an HTML block.
///
/// # Returns
///
/// A semantic paragraph containing the literal HTML block payload.
fn parse_html_block<'a>(events: &mut Peekable<Parser<'a>>) -> View {
    let mut content = InlineText::new();

    for event in events.by_ref() {
        match event {
            Event::End(TagEnd::HtmlBlock) => break,
            Event::Text(text)
            | Event::Code(text)
            | Event::InlineMath(text)
            | Event::DisplayMath(text)
            | Event::Html(text)
            | Event::InlineHtml(text) => content.push_text(&text, Style::new()),
            Event::FootnoteReference(label) => {
                push_footnote_reference(&mut content, &label, Style::new());
            }
            Event::SoftBreak | Event::HardBreak => content.push_break(),
            Event::TaskListMarker(checked) => {
                push_task_list_marker(&mut content, checked, Style::new());
            }
            Event::Start(_) | Event::End(_) | Event::Rule => {}
        }
    }

    paragraph(content.into_text())
}

/// Converts accumulated direct inline content into a semantic paragraph.
///
/// # Arguments
///
/// * `inline` — Pending styled inline text.
/// * `views` — Destination block sequence for the resulting paragraph.
fn flush_inline_paragraph(inline: &mut InlineText, views: &mut Vec<View>) {
    if inline.has_content() {
        views.push(paragraph(std::mem::take(inline).into_text()));
    }
}

/// Collects styled inline content through a matching closing tag.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned after an opening tag.
/// * `end` — Closing tag that terminates the inline content.
///
/// # Returns
///
/// An owned [`Text`] containing styled spans and retained line breaks.
fn parse_inline<'a>(events: &mut Peekable<Parser<'a>>, end: TagEnd) -> Text<'static> {
    let mut content = InlineText::new();
    parse_inline_events(events, end, Style::new(), &mut content);
    content.into_text()
}

/// Parses inline events into an accumulator using the inherited span style.
///
/// Recursive calls combine nested emphasis, strong, and link modifiers while
/// retaining a single output accumulator.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside inline content.
/// * `end` — Closing tag that terminates the current inline scope.
/// * `style` — Span style inherited by text in the current scope.
/// * `content` — Destination rich-text accumulator.
fn parse_inline_events<'a>(
    events: &mut Peekable<Parser<'a>>,
    end: TagEnd,
    style: Style,
    content: &mut InlineText,
) {
    while let Some(event) = events.next() {
        match event {
            Event::Text(text) => content.push_text(&text, style),
            Event::Code(text) => {
                content.push_text(&text, style.add_modifier(Modifier::REVERSED));
            }
            Event::SoftBreak | Event::HardBreak => content.push_break(),
            Event::End(tag) if tag == end => break,
            Event::Start(Tag::Emphasis) => parse_inline_events(
                events,
                TagEnd::Emphasis,
                style.add_modifier(Modifier::ITALIC),
                content,
            ),
            Event::Start(Tag::Strong) => parse_inline_events(
                events,
                TagEnd::Strong,
                style.add_modifier(Modifier::BOLD),
                content,
            ),
            Event::Start(Tag::Link { dest_url, .. }) => {
                parse_link(events, &dest_url, style, content);
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                parse_image(events, &dest_url, style, content);
            }
            Event::Start(Tag::CodeBlock(_)) => skip_until(events, TagEnd::CodeBlock),
            Event::Start(Tag::HtmlBlock) => skip_until(events, TagEnd::HtmlBlock),
            Event::InlineMath(text)
            | Event::DisplayMath(text)
            | Event::Html(text)
            | Event::InlineHtml(text) => content.push_text(&text, style),
            Event::FootnoteReference(label) => {
                push_footnote_reference(content, &label, style);
            }
            Event::TaskListMarker(checked) => {
                push_task_list_marker(content, checked, style);
            }
            Event::Start(_) | Event::End(_) | Event::Rule => {}
        }
    }
}

/// Parses a Markdown image into deterministic descriptive terminal text.
///
/// No image view is constructed, so remote and local destinations are never
/// opened. Parsed alt content is flattened to visible text and combined with
/// the source using a stable `Image:` label.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside an image.
/// * `destination` — Parsed image source URL or path.
/// * `style` — Span style inherited from the surrounding inline scope.
/// * `content` — Destination rich-text accumulator.
fn parse_image<'a>(
    events: &mut Peekable<Parser<'a>>,
    destination: &str,
    style: Style,
    content: &mut InlineText,
) {
    let mut alt = InlineText::new();
    parse_inline_events(events, TagEnd::Image, style, &mut alt);
    let alt = alt.plain_text();
    let fallback = match (alt.is_empty(), destination.is_empty()) {
        (false, false) => format!("Image: {alt} ({destination})"),
        (false, true) => format!("Image: {alt}"),
        (true, false) => format!("Image: {destination}"),
        (true, true) => "Image".to_owned(),
    };
    content.push_text(&fallback, style);
}

/// Appends a readable footnote reference when such an event is enabled.
///
/// # Arguments
///
/// * `content` — Destination rich-text accumulator.
/// * `label` — Parsed footnote label.
/// * `style` — Span style inherited from the surrounding inline scope.
fn push_footnote_reference(content: &mut InlineText, label: &str, style: Style) {
    content.push_text(&format!("[^{label}]"), style);
}

/// Appends a readable checkbox marker when task-list events are enabled.
///
/// # Arguments
///
/// * `content` — Destination rich-text accumulator.
/// * `checked` — Whether the parsed task marker is checked.
/// * `style` — Span style inherited from the surrounding inline scope.
fn push_task_list_marker(content: &mut InlineText, checked: bool, style: Style) {
    content.push_text(if checked { "[x] " } else { "[ ] " }, style);
}

/// Parses a Markdown link and appends a terminal-readable destination.
///
/// Link labels are underlined and retain surrounding emphasis or strong
/// modifiers. Destinations are appended only when the visible label does not
/// already expose the exact URL or an email address without its `mailto:`
/// scheme.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside a link.
/// * `destination` — Parsed link destination.
/// * `style` — Span style inherited from the surrounding inline scope.
/// * `content` — Destination rich-text accumulator.
fn parse_link<'a>(
    events: &mut Peekable<Parser<'a>>,
    destination: &str,
    style: Style,
    content: &mut InlineText,
) {
    let link_style = style.add_modifier(Modifier::UNDERLINED);
    let mut link = InlineText::new();
    parse_inline_events(events, TagEnd::Link, link_style, &mut link);

    let label = link.plain_text();
    if !link_destination_is_visible(&label, destination) {
        if label.is_empty() {
            link.push_text(destination, link_style);
        } else {
            link.push_text(&format!(" ({destination})"), link_style);
        }
    }

    content.append(link);
}

/// Returns whether a link label already displays its destination.
///
/// Email autolinks omit the `mailto:` scheme from their visible label, so that
/// prefix is ignored for the comparison.
///
/// # Arguments
///
/// * `label` — Visible unstyled link-label content.
/// * `destination` — Parsed link destination.
///
/// # Returns
///
/// A boolean indicating whether appending the destination would duplicate it.
fn link_destination_is_visible(label: &str, destination: &str) -> bool {
    label == destination || destination.strip_prefix("mailto:") == Some(label)
}

/// Converts one fenced or indented Markdown code block into a code-block view.
///
/// Fenced blocks use only the first whitespace-delimited info-string token as
/// their displayed language and highlighter selector. Indented and empty-info
/// blocks remain unlabeled and use plain retained source lines.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside the code block.
/// * `kind` — Parsed fenced info string or indented-block marker.
///
/// # Returns
///
/// A [`View::CodeBlock`] retaining the parsed source and language selection.
fn parse_code_block<'a>(events: &mut Peekable<Parser<'a>>, kind: CodeBlockKind<'a>) -> View {
    let mut source = String::new();
    for event in events.by_ref() {
        match event {
            Event::Text(text) | Event::Code(text) => source.push_str(&text),
            Event::SoftBreak | Event::HardBreak => source.push('\n'),
            Event::End(TagEnd::CodeBlock) => break,
            _ => {}
        }
    }

    let language = match kind {
        CodeBlockKind::Indented => None,
        CodeBlockKind::Fenced(info) => info.split_whitespace().next().map(str::to_owned),
    };
    let view = code_block(source);
    match language {
        Some(language) => view.language(language),
        None => view,
    }
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
fn heading(level: HeadingLevel, content: Text<'static>) -> View {
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
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::{RenderCtx, Result};

    /// Renders Markdown into fixed terminal rows for fallback assertions.
    ///
    /// # Arguments
    ///
    /// * `source` — CommonMark source to convert and render.
    /// * `width` — Test terminal width in cells.
    /// * `height` — Test terminal height in cells.
    ///
    /// # Returns
    ///
    /// A [`Vec`] containing rendered terminal symbols grouped by row.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] if terminal or view rendering fails.
    fn rendered_markdown_lines(source: &str, width: u16, height: u16) -> Result<Vec<String>> {
        let view = markdown_to_view(source);
        let mut terminal = Terminal::new(TestBackend::new(width, height))?;
        let mut render_result = Ok(());

        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            render_result = view.render(&mut ctx);
        })?;
        render_result?;

        let cells = terminal.backend().buffer().content();
        Ok(cells
            .chunks(usize::from(width))
            .map(|row| row.iter().map(|cell| cell.symbol()).collect())
            .collect())
    }

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
    /// - Unicode remains intact and inline code uses reverse-video styling.
    #[test]
    fn markdown_preserves_paragraph_breaks_and_unicode() {
        let source = "Soft\nbreak  \nhard 界 `code`\n";

        assert_eq!(
            markdown_to_view(source),
            column([paragraph(Text::from(vec![
                Line::raw("Soft"),
                Line::raw("break"),
                Line::from(vec![
                    Span::raw("hard 界 "),
                    Span::styled("code", Style::new().add_modifier(Modifier::REVERSED),),
                ]),
            ]))]),
        );
    }

    /// Verifies fenced Markdown code selects highlighting from its first info token.
    ///
    /// # Example Under Test
    ///
    /// ````text
    /// ```rust ignored
    /// fn main() {}
    /// ```
    ///
    /// ```rs
    /// let value = true;
    /// ```
    ///
    /// ```unknown-language
    /// plain
    /// ```
    /// ````
    ///
    /// # Assertions
    ///
    /// - The first fence uses `rust` rather than the trailing info-string token.
    /// - The `rs` alias selects the same bundled Rust grammar.
    /// - Unknown languages retain their label and fall back to plain source.
    /// - Source-ending newlines remain available to wrapped code rendering.
    #[test]
    fn markdown_maps_fenced_code_languages_and_fallbacks() {
        let source = concat!(
            "```rust ignored\n",
            "fn main() {}\n",
            "```\n\n",
            "```rs\n",
            "let value = true;\n",
            "```\n\n",
            "```unknown-language\n",
            "plain\n",
            "```\n",
        );

        assert_eq!(
            markdown_to_view(source),
            column([
                code_block("fn main() {}\n").language("rust"),
                code_block("let value = true;\n").language("rs"),
                code_block("plain\n").language("unknown-language"),
            ]),
        );
    }

    /// Verifies empty fenced and indented Markdown code become plain code blocks.
    ///
    /// # Example Under Test
    ///
    /// ````text
    /// ```
    /// ```
    ///
    ///     plain 界
    /// ````
    ///
    /// # Assertions
    ///
    /// - An empty fence produces an empty unlabeled code block.
    /// - Indented Unicode source produces an unlabeled plain code block.
    /// - Both mappings preserve the code-block builder defaults.
    #[test]
    fn markdown_maps_empty_and_indented_code_blocks() {
        let source = "```\n```\n\n    plain 界\n";

        assert_eq!(
            markdown_to_view(source),
            column([code_block(""), code_block("plain 界\n")]),
        );
    }

    /// Verifies Markdown inline syntax produces composable terminal modifiers.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// *outer **bold 界** tail* and **plain** with `code` plus \*escaped\*.
    /// ```
    ///
    /// # Assertions
    ///
    /// - Emphasis uses italics and strong text uses bold.
    /// - Nested emphasis and strong text combine both modifiers.
    /// - Inline code uses reverse video without changing its content.
    /// - Escaped delimiters remain literal unstyled text.
    /// - Adjacent parser text events coalesce into stable spans.
    #[test]
    fn markdown_styles_nested_inline_syntax_and_escaped_text() {
        let source = "*outer **bold 界** tail* and **plain** with `code` plus \\*escaped\\*.\n";

        assert_eq!(
            markdown_to_view(source),
            column([paragraph(Text::from(Line::from(vec![
                Span::styled("outer ", Style::new().add_modifier(Modifier::ITALIC),),
                Span::styled(
                    "bold 界",
                    Style::new().add_modifier(Modifier::ITALIC | Modifier::BOLD),
                ),
                Span::styled(" tail", Style::new().add_modifier(Modifier::ITALIC),),
                Span::raw(" and "),
                Span::styled("plain", Style::new().add_modifier(Modifier::BOLD)),
                Span::raw(" with "),
                Span::styled("code", Style::new().add_modifier(Modifier::REVERSED)),
                Span::raw(" plus *escaped*.")
            ])))]),
        );
    }

    /// Verifies Markdown links remain readable without terminal link interaction.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Read [the *guide*](https://example.com/guide),
    /// [https://example.com](https://example.com), <https://example.org>,
    /// and <reader@example.com>.
    /// ```
    ///
    /// # Assertions
    ///
    /// - Link labels are underlined and retain nested emphasis.
    /// - A descriptive label is followed by its parenthesized destination.
    /// - URL labels and URL autolinks do not duplicate their destinations.
    /// - Email autolinks do not expose or duplicate the `mailto:` scheme.
    #[test]
    fn markdown_styles_links_and_appends_hidden_destinations() {
        let source = concat!(
            "Read [the *guide*](https://example.com/guide), ",
            "[https://example.com](https://example.com), ",
            "<https://example.org>, and <reader@example.com>.\n",
        );
        let underline = Style::new().add_modifier(Modifier::UNDERLINED);

        assert_eq!(
            markdown_to_view(source),
            column([paragraph(Text::from(Line::from(vec![
                Span::raw("Read "),
                Span::styled("the ", underline),
                Span::styled("guide", underline.add_modifier(Modifier::ITALIC),),
                Span::styled(" (https://example.com/guide)", underline),
                Span::raw(", "),
                Span::styled("https://example.com", underline),
                Span::raw(", "),
                Span::styled("https://example.org", underline),
                Span::raw(", and "),
                Span::styled("reader@example.com", underline),
                Span::raw("."),
            ])))]),
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

    /// Verifies nested blockquotes retain semantic children and visible prefixes.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// > Alpha beta gamma
    /// >
    /// > > Inner
    /// ```
    ///
    /// # Assertions
    ///
    /// - Each quote becomes a left-bordered block with readable padding.
    /// - The outer border remains visible beside wrapped content.
    /// - The nested quote stacks a second border without flattening its child.
    #[test]
    fn markdown_renders_nested_blockquotes_with_wrapped_prefixes() -> Result<()> {
        let source = "> Alpha beta gamma\n>\n> > Inner\n";

        assert_eq!(
            markdown_to_view(source),
            column([block_quote(vec![
                paragraph("Alpha beta gamma"),
                block_quote(vec![paragraph("Inner")]),
            ])]),
        );

        let lines = rendered_markdown_lines(source, 12, 3)?;
        assert!(lines[0].starts_with("│ Alpha beta"));
        assert!(lines[1].starts_with("│ gamma"));
        assert!(lines[2].starts_with("│ │ Inner"));

        Ok(())
    }

    /// Verifies thematic breaks render as width-responsive terminal rules.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// ---
    /// ```
    ///
    /// # Assertions
    ///
    /// - A thematic break maps to a dedicated one-row fallback block.
    /// - A one-cell terminal renders one horizontal rule glyph without panic.
    /// - Wider terminals fill the complete row with rule glyphs.
    #[test]
    fn markdown_renders_thematic_breaks_at_narrow_widths() -> Result<()> {
        assert_eq!(markdown_to_view("---\n"), column([thematic_break()]));
        assert_eq!(rendered_markdown_lines("---\n", 1, 1)?, ["─"]);
        assert_eq!(rendered_markdown_lines("---\n", 6, 1)?, ["──────"]);

        Ok(())
    }

    /// Verifies Markdown images become descriptive text without image loading.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// ![diagram](https://example.com/diagram.png)
    /// ![](local.png)
    /// ![caption]()
    /// ![]()
    /// ```
    ///
    /// # Assertions
    ///
    /// - Alt text and source are both shown when present.
    /// - Source-only and alt-only images remain descriptive.
    /// - An image with neither value still has a readable label.
    /// - Every image maps to text rather than a path-backed image view.
    #[test]
    fn markdown_maps_images_to_descriptive_text() {
        let source = concat!(
            "![diagram](https://example.com/diagram.png)\n\n",
            "![](local.png)\n\n",
            "![caption]()\n\n",
            "![]()\n",
        );

        assert_eq!(
            markdown_to_view(source),
            column([
                paragraph("Image: diagram (https://example.com/diagram.png)"),
                paragraph("Image: local.png"),
                paragraph("Image: caption"),
                paragraph("Image"),
            ]),
        );
    }

    /// Verifies raw HTML remains literal and entities follow CommonMark parsing.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Before <kbd>&amp;</kbd> after.
    ///
    /// <section>
    /// block &amp;
    /// </section>
    ///
    /// Fish &amp; Chips &copy;
    /// ```
    ///
    /// # Assertions
    ///
    /// - Inline HTML tags are retained around decoded CommonMark text.
    /// - Block HTML tags, entities, and source line endings remain literal.
    /// - Entities in ordinary Markdown text decode to their visible characters.
    /// - Following semantic content remains in source order.
    #[test]
    fn markdown_preserves_literal_html_and_entities() {
        let source = concat!(
            "Before <kbd>&amp;</kbd> after.\n\n",
            "<section>\n",
            "block &amp;\n",
            "</section>\n\n",
            "Fish &amp; Chips &copy;\n",
        );

        assert_eq!(
            markdown_to_view(source),
            column([
                paragraph("Before <kbd>&</kbd> after."),
                paragraph(Text::from(vec![
                    Line::raw("<section>"),
                    Line::raw("block &amp;"),
                    Line::raw("</section>"),
                    Line::default(),
                ])),
                paragraph("Fish & Chips ©"),
            ]),
        );
    }

    /// Verifies textual extension events remain readable when encountered.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// - [x] ~~done~~ and $x + y$[^note]
    ///
    /// $$z$$
    ///
    /// [^note]: Detail
    /// ```
    ///
    /// # Assertions
    ///
    /// - Unsupported inline presentation drops styling but retains its payload.
    /// - Task and footnote events receive readable terminal markers.
    /// - Display-math and footnote-definition text stays in source order.
    /// - Production parsing remains limited to CommonMark plus tables.
    #[test]
    fn markdown_preserves_text_from_unsupported_parser_events() {
        let source = concat!(
            "- [x] ~~done~~ and $x + y$[^note]\n\n",
            "$$z$$\n\n",
            "[^note]: Detail\n",
        );
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_MATH
            | Options::ENABLE_FOOTNOTES;
        let mut parser = Parser::new_ext(source, options).peekable();

        assert_eq!(
            column(parse_blocks(&mut parser, None)),
            column([
                unordered_list([list_item([paragraph("[x] done and x + y[^note]")])]),
                paragraph("z"),
                paragraph("Detail"),
            ]),
        );
    }

    /// Verifies semantic blocks and readable fallbacks retain source order.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// # Start
    ///
    /// ---
    ///
    /// ![middle](middle.png)
    ///
    /// <end>
    /// ```
    ///
    /// # Assertions
    ///
    /// - The semantic heading remains first.
    /// - Rule, image, and raw-HTML fallbacks retain their original order.
    #[test]
    fn markdown_preserves_fallback_source_order() {
        let source = "# Start\n\n---\n\n![middle](middle.png)\n\n<end>\n";

        assert_eq!(
            markdown_to_view(source),
            column([
                h1("Start"),
                thematic_break(),
                paragraph("Image: middle (middle.png)"),
                paragraph(Text::from(vec![Line::raw("<end>"), Line::default()])),
            ]),
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
