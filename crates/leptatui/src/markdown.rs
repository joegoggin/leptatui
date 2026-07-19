//! CommonMark parsing and semantic document-view conversion.
//!
//! This module converts pulldown-cmark's balanced event stream into the
//! semantic headings, paragraphs, lists, tables, highlighted code blocks, and
//! styled inline spans exposed by [`crate::view`]. Readable styled-block or
//! text fallbacks retain CommonMark content without dedicated semantic views.
//! In-memory readers are infallible, while explicit file readers finish UTF-8
//! filesystem loading before returning a view. Only the table extension is
//! enabled beyond core CommonMark.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
};

use crate::{
    Borders, CellAlignment, SyntaxTheme, TuiSpacing, TuiStyle, View, block, code_block, column, h1,
    h2, h3, h4, h5, h6, list_item, ordered_list, paragraph, table, table_body, table_cell,
    table_head, table_row, unordered_list,
};

/// Default presentation options applied while converting Markdown documents.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MarkdownOptions {
    /// Bundled syntax theme applied to fenced and indented code blocks.
    syntax_theme: SyntaxTheme,
    /// Whether parsed code blocks display one-based line numbers.
    line_numbers: bool,
}

impl MarkdownOptions {
    /// Sets the bundled syntax theme for parsed code blocks.
    ///
    /// # Arguments
    ///
    /// * `syntax_theme` — Dark or light bundled theme selection.
    ///
    /// # Returns
    ///
    /// A [`MarkdownOptions`] value with the requested syntax theme.
    pub fn syntax_theme(mut self, syntax_theme: SyntaxTheme) -> Self {
        self.syntax_theme = syntax_theme;
        self
    }

    /// Sets default line-number visibility for parsed code blocks.
    ///
    /// # Arguments
    ///
    /// * `line_numbers` — Whether to display one-based logical line numbers.
    ///
    /// # Returns
    ///
    /// A [`MarkdownOptions`] value with the requested line-number behavior.
    pub fn line_numbers(mut self, line_numbers: bool) -> Self {
        self.line_numbers = line_numbers;
        self
    }
}

/// Errors returned while loading Markdown files.
#[derive(Debug, thiserror::Error)]
pub enum MarkdownError {
    /// Reading or decoding a Markdown file failed.
    #[error("failed to read Markdown file `{path}`: {source}", path = .path.display())]
    Read {
        /// Path supplied to the Markdown file reader.
        path: PathBuf,
        /// Underlying filesystem or UTF-8 decoding error.
        #[source]
        source: io::Error,
    },
}

/// Converts CommonMark source into a scrollable semantic document view.
///
/// Uses [`MarkdownOptions::default`] and performs no filesystem access.
///
/// # Arguments
///
/// * `source` — CommonMark source text to parse.
///
/// # Returns
///
/// A [`View::Column`] containing semantic document blocks in source order.
pub fn markdown(source: impl AsRef<str>) -> View {
    markdown_with_options(source, MarkdownOptions::default())
}

/// Converts CommonMark source with explicit presentation options.
///
/// Parsing is infallible and performs no filesystem access.
///
/// # Arguments
///
/// * `source` — CommonMark source text to parse.
/// * `options` — Code-block presentation defaults for the document.
///
/// # Returns
///
/// A [`View::Column`] containing semantic document blocks in source order.
pub fn markdown_with_options(source: impl AsRef<str>, options: MarkdownOptions) -> View {
    let mut parser = Parser::new_ext(source.as_ref(), Options::ENABLE_TABLES);
    column(parse_blocks(&mut parser, None, options))
}

/// Loads a UTF-8 Markdown file into a scrollable semantic document view.
///
/// Uses [`MarkdownOptions::default`] and performs all filesystem access before
/// returning the view.
///
/// # Arguments
///
/// * `path` — Path to the UTF-8 Markdown file to load.
///
/// # Returns
///
/// A [`Result`](std::result::Result) containing the parsed document view.
///
/// # Errors
///
/// Returns [`MarkdownError::Read`] if the path cannot be read as a UTF-8 file.
pub fn markdown_file(path: impl AsRef<Path>) -> Result<View, MarkdownError> {
    markdown_file_with_options(path, MarkdownOptions::default())
}

/// Loads a UTF-8 Markdown file with explicit presentation options.
///
/// All filesystem access completes before the returned view enters render
/// traversal.
///
/// # Examples
///
/// ```no_run
/// use leptatui::{MarkdownOptions, SyntaxTheme, markdown_file_with_options};
///
/// let view = markdown_file_with_options(
///     "README.md",
///     MarkdownOptions::default().syntax_theme(SyntaxTheme::Light),
/// )?;
/// # let _ = view;
/// # Ok::<(), leptatui::MarkdownError>(())
/// ```
///
/// # Arguments
///
/// * `path` — Path to the UTF-8 Markdown file to load.
/// * `options` — Code-block presentation defaults for the document.
///
/// # Returns
///
/// A [`Result`](std::result::Result) containing the parsed document view.
///
/// # Errors
///
/// Returns [`MarkdownError::Read`] if the path cannot be read as a UTF-8 file.
pub fn markdown_file_with_options(
    path: impl AsRef<Path>,
    options: MarkdownOptions,
) -> Result<View, MarkdownError> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|source| MarkdownError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(markdown_with_options(source, options))
}

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

/// Parses semantic blocks until the requested closing tag or end of input.
///
/// Direct inline events are collected into a paragraph because pulldown-cmark
/// omits paragraph tags inside tight list items.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside a block.
/// * `end` — Optional closing tag that terminates the current block sequence.
/// * `options` — Code-block presentation defaults for the document.
///
/// # Returns
///
/// A [`Vec`] containing converted semantic views in event order.
fn parse_blocks<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    end: Option<TagEnd>,
    options: MarkdownOptions,
) -> Vec<View> {
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
                views.push(parse_list(events, start, options));
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
                    options,
                )));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_code_block(events, kind, options));
            }
            Event::Start(Tag::HtmlBlock) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_html_block(events));
            }
            Event::Rule => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(thematic_break());
            }
            Event::End(tag) if Some(tag) == end => {
                flush_inline_paragraph(&mut inline, &mut views);
                break;
            }
            event => parse_inline_event(events, event, Style::new(), &mut inline),
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
fn parse_html_block<'a>(events: &mut impl Iterator<Item = Event<'a>>) -> View {
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
fn parse_inline<'a>(events: &mut impl Iterator<Item = Event<'a>>, end: TagEnd) -> Text<'static> {
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
    events: &mut impl Iterator<Item = Event<'a>>,
    end: TagEnd,
    style: Style,
    content: &mut InlineText,
) {
    while let Some(event) = events.next() {
        if event == Event::End(end) {
            break;
        }
        parse_inline_event(events, event, style, content);
    }
}

/// Parses one inline event into an accumulator using an inherited span style.
///
/// Nested inline tags recursively consume their balanced event scopes. Block
/// tags encountered inside inline content are skipped through their closing
/// tags, while unsupported structural events are ignored.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned after the current event.
/// * `event` — Inline or fallback event to convert.
/// * `style` — Span style inherited by the event's content.
/// * `content` — Destination rich-text accumulator.
fn parse_inline_event<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    event: Event<'a>,
    style: Style,
    content: &mut InlineText,
) {
    match event {
        Event::Text(text)
        | Event::InlineMath(text)
        | Event::DisplayMath(text)
        | Event::Html(text)
        | Event::InlineHtml(text) => content.push_text(&text, style),
        Event::Code(text) => {
            content.push_text(&text, style.add_modifier(Modifier::REVERSED));
        }
        Event::SoftBreak | Event::HardBreak => content.push_break(),
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
        Event::FootnoteReference(label) => {
            push_footnote_reference(content, &label, style);
        }
        Event::TaskListMarker(checked) => {
            push_task_list_marker(content, checked, style);
        }
        Event::Start(_) | Event::End(_) | Event::Rule => {}
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
    events: &mut impl Iterator<Item = Event<'a>>,
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
/// modifiers. Non-empty destinations are appended only when the visible label
/// does not already expose the exact URL or an email address without its
/// `mailto:` scheme.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside a link.
/// * `destination` — Parsed link destination.
/// * `style` — Span style inherited from the surrounding inline scope.
/// * `content` — Destination rich-text accumulator.
fn parse_link<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    destination: &str,
    style: Style,
    content: &mut InlineText,
) {
    let link_style = style.add_modifier(Modifier::UNDERLINED);
    let mut link = InlineText::new();
    parse_inline_events(events, TagEnd::Link, link_style, &mut link);

    let label = link.plain_text();
    if !destination.is_empty() && !link_destination_is_visible(&label, destination) {
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
/// * `options` — Presentation defaults applied to the parsed code block.
///
/// # Returns
///
/// A [`View::CodeBlock`] retaining the parsed source and language selection.
fn parse_code_block<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    kind: CodeBlockKind<'a>,
    options: MarkdownOptions,
) -> View {
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
    let view = code_block(source)
        .line_numbers(options.line_numbers)
        .syntax_theme(options.syntax_theme);
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
/// * `options` — Code-block presentation defaults for nested list content.
///
/// # Returns
///
/// A semantic ordered or unordered list retaining item order and nesting.
fn parse_list<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    start: Option<u64>,
    options: MarkdownOptions,
) -> View {
    let mut items = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::Item) => {
                items.push(list_item(parse_blocks(events, Some(TagEnd::Item), options)));
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
fn parse_table<'a>(events: &mut impl Iterator<Item = Event<'a>>, alignments: &[Alignment]) -> View {
    let mut header_rows = Vec::new();
    let mut body_rows = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::TableHead) => {
                header_rows.push(parse_table_cells(events, alignments, TagEnd::TableHead));
            }
            Event::Start(Tag::TableRow) => {
                body_rows.push(parse_table_cells(events, alignments, TagEnd::TableRow));
            }
            Event::End(TagEnd::Table) => break,
            _ => {}
        }
    }

    table([table_head(header_rows), table_body(body_rows)])
}

/// Parses CommonMark table cells into one semantic header or body row.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside a table row.
/// * `alignments` — Parsed alignment for each source column.
/// * `end` — Closing tag that terminates the header or body row.
///
/// # Returns
///
/// A semantic table-row view containing aligned cells.
fn parse_table_cells<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    alignments: &[Alignment],
    end: TagEnd,
) -> View {
    let mut cells = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::TableCell) => {
                let alignment = alignment_at(alignments, cells.len());
                cells
                    .push(table_cell(parse_inline(events, TagEnd::TableCell)).alignment(alignment));
            }
            Event::End(tag) if tag == end => break,
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
fn skip_until<'a>(events: &mut impl Iterator<Item = Event<'a>>, end: TagEnd) {
    for event in events.by_ref() {
        if event == Event::End(end) {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error as _,
        sync::atomic::{AtomicU64, Ordering},
    };

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::{RenderCtx, Result};

    /// Returns a unique temporary directory path for Markdown reader fixtures.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable suffix identifying the fixture purpose.
    ///
    /// # Returns
    ///
    /// A [`PathBuf`] below the process temporary directory.
    fn markdown_fixture_dir(name: &str) -> PathBuf {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

        std::env::temp_dir().join(format!(
            "leptatui-markdown-{}-{}-{name}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    /// Returns code-block options from a single-block Markdown document.
    ///
    /// # Arguments
    ///
    /// * `view` — Parsed document expected to contain one code block.
    ///
    /// # Returns
    ///
    /// A tuple containing line-number visibility and the syntax theme.
    fn parsed_code_block_options(view: &View) -> (bool, SyntaxTheme) {
        let View::Column { children, .. } = view else {
            panic!("expected Markdown column, got {view:?}");
        };
        let [
            View::CodeBlock {
                line_numbers,
                syntax_theme,
                ..
            },
        ] = children.as_slice()
        else {
            panic!("expected one Markdown code block, got {children:?}");
        };

        (*line_numbers, *syntax_theme)
    }

    /// Returns scroll offset and maximum offset from a Markdown document.
    ///
    /// # Arguments
    ///
    /// * `view` — Parsed Markdown column whose scroll metadata is inspected.
    ///
    /// # Returns
    ///
    /// A tuple containing the current and maximum vertical scroll offsets.
    fn markdown_scroll_state(view: &View) -> (u16, u16) {
        let View::Column { metadata, .. } = view else {
            panic!("expected Markdown column, got {view:?}");
        };

        (metadata.scroll_offset(), metadata.max_scroll_offset())
    }

    /// Verifies a Markdown reader error retains its path and I/O source.
    ///
    /// # Arguments
    ///
    /// * `error` — Reader error returned for the failing path.
    /// * `expected_path` — Exact path expected in the public error variant.
    ///
    /// # Returns
    ///
    /// An [`io::Error`] containing the preserved underlying failure.
    fn assert_markdown_read_error(error: MarkdownError, expected_path: &Path) -> io::Error {
        let diagnostic = error.to_string();
        assert!(diagnostic.contains(&expected_path.display().to_string()));
        assert!(error.source().is_some());

        let MarkdownError::Read { path, source } = error;
        assert_eq!(path, expected_path);
        source
    }

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
        let view = markdown(source);
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

    /// Verifies in-memory Markdown readers apply default and custom options.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// markdown("```rust\nfn main() {}\n```")
    /// markdown_with_options(source, light theme + line numbers)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Both in-memory readers return document views without failure.
    /// - Default code blocks use the dark theme without line numbers.
    /// - Custom options apply the light theme and enable line numbers.
    /// - An owned source string is accepted by the option-bearing reader.
    #[test]
    fn markdown_reader_apis_apply_default_and_custom_options() {
        let source = "```rust\nfn main() {}\n```\n";
        let default = markdown(source);
        assert_eq!(
            parsed_code_block_options(&default),
            (false, SyntaxTheme::Dark)
        );

        let options = MarkdownOptions::default()
            .syntax_theme(SyntaxTheme::Light)
            .line_numbers(true);
        let owned_source = source.to_owned();
        let configured = markdown_with_options(owned_source, options);
        assert_eq!(
            parsed_code_block_options(&configured),
            (true, SyntaxTheme::Light)
        );
    }

    /// Verifies Markdown file readers synchronously load UTF-8 source.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// markdown_file("guide.md")
    /// markdown_file_with_options("guide.md", light theme + line numbers)
    /// ```
    ///
    /// # Assertions
    ///
    /// - The UTF-8 fixture writes and both file readers load it successfully.
    /// - The default file reader matches the in-memory default reader.
    /// - The option-bearing file reader applies its code-block defaults.
    /// - The fixture directory is removed after verification.
    #[test]
    fn markdown_file_reader_apis_load_utf8_source() {
        let fixture_dir = markdown_fixture_dir("readers");
        let fixture_path = fixture_dir.join("guide.md");
        let source = "```rust\nfn main() {}\n```\n";
        fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
        fs::write(&fixture_path, source).expect("Markdown fixture should be written");

        let default = markdown_file(&fixture_path).expect("default file reader should succeed");
        assert_eq!(default, markdown(source));

        let options = MarkdownOptions::default()
            .syntax_theme(SyntaxTheme::Light)
            .line_numbers(true);
        let configured = markdown_file_with_options(&fixture_path, options)
            .expect("configured file reader should succeed");
        assert_eq!(
            parsed_code_block_options(&configured),
            (true, SyntaxTheme::Light)
        );

        fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
    }

    /// Verifies Markdown file errors retain path-aware I/O diagnostics.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// missing.md
    /// directory.md/
    /// invalid-utf8.md containing FF FE
    /// ```
    ///
    /// # Assertions
    ///
    /// - Missing paths report [`io::ErrorKind::NotFound`].
    /// - Directory paths return their platform I/O failure instead of parsing.
    /// - Invalid UTF-8 reports [`io::ErrorKind::InvalidData`].
    /// - Every error retains the exact path, includes it in display output, and chains its cause.
    /// - The fixture directory is removed after verification.
    #[test]
    fn markdown_file_errors_preserve_paths_and_io_causes() {
        let fixture_dir = markdown_fixture_dir("errors");
        let directory_path = fixture_dir.join("directory.md");
        let invalid_utf8_path = fixture_dir.join("invalid-utf8.md");
        let missing_path = fixture_dir.join("missing.md");
        fs::create_dir_all(&directory_path).expect("directory fixture should be created");
        fs::write(&invalid_utf8_path, [0xff, 0xfe])
            .expect("invalid UTF-8 fixture should be written");

        let missing = markdown_file(&missing_path).expect_err("missing path should fail");
        assert_eq!(
            assert_markdown_read_error(missing, &missing_path).kind(),
            io::ErrorKind::NotFound
        );

        let directory = markdown_file(&directory_path).expect_err("directory path should fail");
        assert_ne!(
            assert_markdown_read_error(directory, &directory_path).kind(),
            io::ErrorKind::NotFound
        );

        let invalid_utf8 =
            markdown_file(&invalid_utf8_path).expect_err("invalid UTF-8 should fail");
        assert_eq!(
            assert_markdown_read_error(invalid_utf8, &invalid_utf8_path).kind(),
            io::ErrorKind::InvalidData
        );

        fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
    }

    /// Verifies in-memory Markdown rendering never interprets source as a path.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// markdown("/temporary/missing.md")
    /// ```
    ///
    /// # Assertions
    ///
    /// - The path does not exist before or after conversion and rendering.
    /// - The path-like source becomes an ordinary Markdown paragraph.
    /// - Rendering succeeds without filesystem access.
    #[test]
    fn markdown_source_rendering_performs_no_filesystem_io() -> Result<()> {
        let missing_path = markdown_fixture_dir("no-io").join("missing.md");
        let source = missing_path.display().to_string();
        assert!(!missing_path.exists());

        let view = markdown(&source);
        assert_eq!(view, column([paragraph(source)]));
        let mut terminal = Terminal::new(TestBackend::new(80, 2))?;
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            render_result = view.render(&mut ctx);
        })?;
        render_result?;

        assert!(!missing_path.exists());
        Ok(())
    }

    /// Verifies Markdown documents use the existing vertical scroll commands.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// ten Markdown paragraphs rendered into a 3-row terminal
    /// Down, Up, PageDown, PageUp, G, gg
    /// ```
    ///
    /// # Assertions
    ///
    /// - Rendering establishes an overflowing scroll range on the document column.
    /// - Arrow keys move one row down and up.
    /// - Page keys move five rows down and up.
    /// - `G` reaches the maximum offset and `gg` returns to zero.
    #[test]
    fn markdown_documents_use_existing_vertical_scroll_keys() -> Result<()> {
        let source = (1..=10)
            .map(|index| format!("Paragraph {index}."))
            .collect::<Vec<_>>()
            .join("\n\n");
        let mut view = markdown(source);
        let mut terminal = Terminal::new(TestBackend::new(20, 3))?;
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            render_result = view.render(&mut ctx);
        })?;
        render_result?;

        let (offset, max_offset) = markdown_scroll_state(&view);
        assert_eq!(offset, 0);
        assert!(max_offset >= 6);

        view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
        assert_eq!(markdown_scroll_state(&view).0, 1);
        view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?;
        assert_eq!(markdown_scroll_state(&view).0, 0);

        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?;
        assert_eq!(markdown_scroll_state(&view).0, 5);
        view.handle_key_event(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE))?;
        assert_eq!(markdown_scroll_state(&view).0, 0);

        view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?;
        assert_eq!(markdown_scroll_state(&view).0, max_offset);
        view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
        view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
        assert_eq!(markdown_scroll_state(&view).0, 0);

        Ok(())
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
            markdown(source),
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
            markdown(source),
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
            markdown(source),
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
            markdown(source),
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
            markdown(source),
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
    /// and <reader@example.com>, plus [empty]().
    /// ```
    ///
    /// # Assertions
    ///
    /// - Link labels are underlined and retain nested emphasis.
    /// - A descriptive label is followed by its parenthesized destination.
    /// - URL labels and URL autolinks do not duplicate their destinations.
    /// - Email autolinks do not expose or duplicate the `mailto:` scheme.
    /// - Links with empty destinations do not display empty parentheses.
    #[test]
    fn markdown_styles_links_and_appends_hidden_destinations() {
        let source = concat!(
            "Read [the *guide*](https://example.com/guide), ",
            "[https://example.com](https://example.com), ",
            "<https://example.org>, and <reader@example.com>, plus [empty]().\n",
        );
        let underline = Style::new().add_modifier(Modifier::UNDERLINED);

        assert_eq!(
            markdown(source),
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
                Span::raw(", plus "),
                Span::styled("empty", underline),
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
            markdown(source),
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
            markdown(source),
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
            markdown(source),
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
        assert_eq!(markdown("---\n"), column([thematic_break()]));
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
            markdown(source),
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
            markdown(source),
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
        let mut parser = Parser::new_ext(source, options);

        assert_eq!(
            column(parse_blocks(&mut parser, None, MarkdownOptions::default(),)),
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
            markdown(source),
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
            markdown(source),
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
        assert_eq!(markdown(""), column([]));
    }
}
