//! CommonMark parsing and semantic document-view conversion.
//!
//! This module converts pulldown-cmark's balanced event stream into the
//! semantic headings, paragraphs, lists, tables, highlighted code blocks, and
//! styled inline spans exposed by [`crate::view`]. Readable styled-block or
//! text fallbacks retain CommonMark content without dedicated semantic views.
//! In-memory and explicit file readers are infallible; file failures become
//! path-aware semantic fallback content. File-backed views navigate local
//! Markdown targets and heading fragments in-app with cached page history.
//! The compatibility promise is core CommonMark plus tables. Optional GFM
//! extensions are deferred. Links retain focusable target metadata, while
//! images become descriptive text without fetching local or remote targets.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use percent_encoding::percent_decode_str;
use pulldown_cmark::{
    Alignment, CodeBlockKind, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd,
};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
};

use crate::{
    Borders, CellAlignment, SyntaxTheme, TuiSpacing, TuiStyle, View, block, code_block, column, h1,
    h2, h3, h4, h5, h6, list_item, ordered_list, paragraph, table, table_body, table_cell,
    table_head, table_row, unordered_list,
};

use crate::view::{InlineLink, LinkTarget, LinkedSpan, RichText};

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

/// Stateful, file-backed Markdown document boundary.
///
/// A Markdown view keeps previously visited pages in memory so back and
/// forward navigation restore their exact focus and scroll state. Construct
/// one with [`markdown_file`] or [`markdown_file_with_options`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownView {
    root_path: PathBuf,
    options: MarkdownOptions,
    current: MarkdownPage,
    back: Vec<MarkdownPage>,
    forward: Vec<MarkdownPage>,
}

/// One cached page in a [`MarkdownView`] navigation history.
#[derive(Clone, Debug, Eq, PartialEq)]
struct MarkdownPage {
    path: PathBuf,
    document: Box<View>,
}

impl MarkdownView {
    /// Returns the path of the currently displayed Markdown page.
    ///
    /// # Returns
    ///
    /// A [`Path`] identifying the current page, including failed load targets.
    pub fn current_path(&self) -> &Path {
        &self.current.path
    }

    /// Returns whether a cached page is available in back history.
    ///
    /// # Returns
    ///
    /// `true` when Shift+H can restore a previous page.
    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    /// Returns whether a cached page is available in forward history.
    ///
    /// # Returns
    ///
    /// `true` when Shift+L can restore a forward page.
    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Creates a file-backed Markdown boundary rooted at `path`.
    fn new(path: &Path, options: MarkdownOptions) -> Self {
        let root_path = absolute_path(path);
        let current = load_markdown_page(root_path.clone(), options, None);
        Self {
            root_path,
            options,
            current,
            back: Vec::new(),
            forward: Vec::new(),
        }
    }

    /// Returns the current parsed document.
    pub(crate) fn document(&self) -> &View {
        &self.current.document
    }

    /// Returns the current parsed document mutably.
    pub(crate) fn document_mut(&mut self) -> &mut View {
        &mut self.current.document
    }

    /// Returns whether this state belongs to the same declarative root.
    pub(crate) fn can_reconcile_from(&self, previous: &Self) -> bool {
        self.root_path == previous.root_path && self.options == previous.options
    }

    /// Navigates to the focused in-app Markdown target, if one exists.
    pub(crate) fn navigate_focused_link(&mut self) -> bool {
        let Some(LinkTarget::Markdown { path, fragment }) =
            self.current.document.focused_link_target().cloned()
        else {
            return false;
        };

        let next = load_markdown_page(path, self.options, fragment.as_deref());
        let previous = std::mem::replace(&mut self.current, next);
        self.back.push(previous);
        self.forward.clear();
        true
    }

    /// Restores the most recent cached page from back history.
    pub(crate) fn go_back(&mut self) -> bool {
        let Some(previous) = self.back.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.current, previous);
        self.forward.push(current);
        true
    }

    /// Restores the next cached page from forward history.
    pub(crate) fn go_forward(&mut self) -> bool {
        let Some(next) = self.forward.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.current, next);
        self.back.push(current);
        true
    }
}

/// Per-document context used while parsing links and heading anchors.
struct MarkdownParseContext<'a> {
    link_base: &'a Path,
    source_path: Option<&'a Path>,
    heading_counts: HashMap<String, usize>,
}

impl<'a> MarkdownParseContext<'a> {
    /// Creates parsing context for in-memory or file-backed Markdown.
    fn new(link_base: &'a Path, source_path: Option<&'a Path>) -> Self {
        Self {
            link_base,
            source_path,
            heading_counts: HashMap::new(),
        }
    }

    /// Returns the unique GitHub-style slug for one heading.
    fn heading_slug(&mut self, content: &RichText) -> String {
        let visible = content
            .text()
            .lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();
        let base = github_heading_slug(&visible);
        let count = self.heading_counts.entry(base.clone()).or_default();
        let slug = if *count == 0 {
            base
        } else {
            format!("{base}-{count}")
        };
        *count = count.saturating_add(1);
        slug
    }

    /// Classifies a parsed Markdown link for this document boundary.
    fn link_target(&self, link_type: LinkType, destination: &str) -> LinkTarget {
        if link_type == LinkType::Email && !destination.starts_with("mailto:") {
            return LinkTarget::Url(format!("mailto:{destination}"));
        }

        let ordinary = LinkTarget::from(destination);
        if matches!(ordinary, LinkTarget::Url(_)) {
            return ordinary;
        }

        if let Some(source_path) = self.source_path {
            let (path, fragment) = destination
                .split_once('#')
                .map_or((destination, None), |(path, fragment)| {
                    (path, Some(fragment))
                });
            if path.is_empty() {
                if let Some(fragment) = fragment.filter(|fragment| !fragment.is_empty()) {
                    return LinkTarget::Markdown {
                        path: source_path.to_path_buf(),
                        fragment: Some(fragment.to_owned()),
                    };
                }
            } else if is_markdown_path(Path::new(path)) {
                return LinkTarget::Markdown {
                    path: absolute_path_from(Path::new(path), self.link_base),
                    fragment: fragment
                        .filter(|fragment| !fragment.is_empty())
                        .map(str::to_owned),
                };
            }
        }

        ordinary.resolve_against(self.link_base)
    }
}

/// Converts CommonMark source into a scrollable semantic document view.
///
/// Uses [`MarkdownOptions::default`] and performs no filesystem access.
///
/// # Examples
///
/// ```
/// use leptatui::markdown;
///
/// let document = markdown("# Guide\n\nRead **semantic** terminal documents.");
/// # let _ = document;
/// ```
///
/// # Arguments
///
/// * `source` — CommonMark source text to parse.
///
/// # Returns
///
/// A [`View::Column`] containing semantic document blocks separated by empty
/// terminal rows in source order.
pub fn markdown(source: impl AsRef<str>) -> View {
    markdown_with_options(source, MarkdownOptions::default())
}

/// Converts CommonMark source with explicit presentation options.
///
/// Parsing is infallible and performs no filesystem access.
///
/// # Examples
///
/// ```
/// use leptatui::{MarkdownOptions, SyntaxTheme, markdown_with_options};
///
/// let source = "```rust\nfn main() {}\n```";
/// let document = markdown_with_options(
///     source,
///     MarkdownOptions::default()
///         .syntax_theme(SyntaxTheme::Dark)
///         .line_numbers(true),
/// );
/// # let _ = document;
/// ```
///
/// # Arguments
///
/// * `source` — CommonMark source text to parse.
/// * `options` — Code-block presentation defaults for the document.
///
/// # Returns
///
/// A [`View::Column`] containing semantic document blocks separated by empty
/// terminal rows in source order.
pub fn markdown_with_options(source: impl AsRef<str>, options: MarkdownOptions) -> View {
    let link_base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    markdown_with_options_and_source(source.as_ref(), options, &link_base, None)
}

/// Converts CommonMark source using an explicit link base directory.
///
/// # Arguments
///
/// * `source` — CommonMark source text to parse.
/// * `options` — Code-block presentation defaults for the document.
/// * `link_base` — Directory used to resolve relative local targets.
/// * `source_path` — File path used to enable in-app Markdown navigation.
///
/// # Returns
///
/// A [`View::Column`] containing semantic document blocks.
fn markdown_with_options_and_source(
    source: &str,
    options: MarkdownOptions,
    link_base: &Path,
    source_path: Option<&Path>,
) -> View {
    let mut parser = Parser::new_ext(source, Options::ENABLE_TABLES);
    let mut context = MarkdownParseContext::new(link_base, source_path);
    column(parse_blocks(&mut parser, None, options, &mut context))
}

/// Loads a UTF-8 Markdown file into a scrollable semantic document view.
///
/// Uses [`MarkdownOptions::default`] and loads the initial file before
/// returning the view. Activating a local Markdown link synchronously loads its
/// target into the same file-backed boundary.
///
/// # Examples
///
/// ```
/// use leptatui::markdown_file;
///
/// let document = markdown_file("README.md");
/// # let _ = document;
/// ```
///
/// # Arguments
///
/// * `path` — Path to the UTF-8 Markdown file to load.
///
/// # Returns
///
/// A [`View::Markdown`] boundary containing the parsed document, navigation
/// history, or a path-aware fallback when the file cannot be read as UTF-8.
pub fn markdown_file(path: impl AsRef<Path>) -> View {
    markdown_file_with_options(path, MarkdownOptions::default())
}

/// Loads a UTF-8 Markdown file with explicit presentation options.
///
/// The initial filesystem load completes before the returned view enters
/// render traversal. Later local Markdown navigation loads during key-event
/// handling, never during rendering.
///
/// # Examples
///
/// ```no_run
/// use leptatui::{MarkdownOptions, SyntaxTheme, markdown_file_with_options};
///
/// let view = markdown_file_with_options(
///     "README.md",
///     MarkdownOptions::default()
///         .syntax_theme(SyntaxTheme::Light)
///         .line_numbers(true),
/// );
/// # let _ = view;
/// ```
///
/// # Arguments
///
/// * `path` — Path to the UTF-8 Markdown file to load.
/// * `options` — Code-block presentation defaults for the document.
///
/// # Returns
///
/// A [`View::Markdown`] boundary containing the parsed document or a
/// path-aware fallback paragraph when the file cannot be read as UTF-8.
pub fn markdown_file_with_options(path: impl AsRef<Path>, options: MarkdownOptions) -> View {
    View::Markdown {
        state: MarkdownView::new(path.as_ref(), options),
    }
}

/// Returns an absolute path without requiring the target to exist.
fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        base.join(path)
    }
}

/// Resolves `path` against `base` without requiring the target to exist.
fn absolute_path_from(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

/// Returns whether a local path names a supported Markdown file extension.
fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

/// Produces the base anchor used for GitHub-style heading fragments.
fn github_heading_slug(heading: &str) -> String {
    let mut slug = String::new();

    for character in heading.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() || character == '-' || character == '_' {
            slug.push(character);
        } else if character.is_whitespace() {
            slug.push('-');
        }
    }

    slug
}

/// Normalizes a percent-encoded fragment for heading-id comparison.
fn normalized_fragment(fragment: &str) -> String {
    percent_decode_str(fragment)
        .decode_utf8_lossy()
        .chars()
        .flat_map(char::to_lowercase)
        .collect()
}

/// Loads one page, retaining read failures as navigable in-app content.
fn load_markdown_page(
    path: PathBuf,
    options: MarkdownOptions,
    fragment: Option<&str>,
) -> MarkdownPage {
    let mut document = match fs::read_to_string(&path) {
        Ok(source) => {
            let link_base = path.parent().unwrap_or_else(|| Path::new("."));
            markdown_with_options_and_source(&source, options, link_base, Some(&path))
        }
        Err(error) => column([paragraph(format!(
            "failed to read Markdown file `{}`: {error}",
            path.display()
        ))]),
    };

    if let Some(fragment) = fragment {
        document.request_scroll_to_id(&normalized_fragment(fragment));
    }

    MarkdownPage {
        path,
        document: Box::new(document),
    }
}

/// Accumulates owned rich-text lines while parsing inline Markdown events.
///
/// Adjacent content with identical styles is coalesced so parser event
/// boundaries do not leak into the semantic view tree. An explicit content
/// flag distinguishes untouched tight-list content from a deliberately empty
/// line created by a Markdown break.
struct InlineText {
    /// Styled spans grouped by logical output line.
    lines: Vec<Vec<ParsedInlineSpan>>,
    /// Link targets retained in source order.
    links: Vec<LinkTarget>,
    /// Whether the parser emitted text or a line break into this accumulator.
    has_content: bool,
}

/// One parsed inline span and its optional owning link index.
struct ParsedInlineSpan {
    /// Visible Ratatui span.
    span: Span<'static>,
    /// Index into [`InlineText::links`] when this span is a link label.
    link: Option<usize>,
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
            links: Vec::new(),
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
        self.push_text_for_link(content, style, None);
    }

    /// Appends styled text associated with an optional link.
    ///
    /// # Arguments
    ///
    /// * `content` — Text emitted by the Markdown parser.
    /// * `style` — Ratatui style applied to the appended text.
    /// * `link` — Optional source-order link index owning the text.
    fn push_text_for_link(&mut self, content: &str, style: Style, link: Option<usize>) {
        if content.is_empty() {
            return;
        }

        self.has_content = true;
        let mut parts = content.split('\n').peekable();
        while let Some(part) = parts.next() {
            if !part.is_empty() {
                self.push_span(part, style, link);
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

    /// Registers a link target and returns its source-order index.
    ///
    /// # Arguments
    ///
    /// * `target` — Link target to retain.
    ///
    /// # Returns
    ///
    /// A [`usize`] index identifying the retained link.
    fn push_link(&mut self, target: LinkTarget) -> usize {
        let index = self.links.len();
        self.links.push(target);
        index
    }

    /// Returns whether visible text has been assigned to a link index.
    ///
    /// # Arguments
    ///
    /// * `link` — Source-order link index to inspect.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether any span belongs to the link.
    fn link_has_text(&self, link: usize) -> bool {
        self.lines
            .iter()
            .flatten()
            .any(|span| span.link == Some(link))
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
                    .map(|span| span.span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Converts accumulated lines into owned Ratatui rich text.
    ///
    /// # Returns
    ///
    /// A [`RichText`] containing every logical line, styled span, and link.
    fn into_rich_text(self) -> RichText {
        let mut linked_spans = vec![Vec::new(); self.links.len()];
        let lines = self
            .lines
            .into_iter()
            .enumerate()
            .map(|(line_index, line)| {
                Line::from(
                    line.into_iter()
                        .enumerate()
                        .map(|(span_index, parsed)| {
                            if let Some(link_index) = parsed.link
                                && let Some(positions) = linked_spans.get_mut(link_index)
                            {
                                positions.push(LinkedSpan {
                                    line: line_index,
                                    span: span_index,
                                });
                            }
                            parsed.span
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        let links = self
            .links
            .into_iter()
            .zip(linked_spans)
            .map(|(target, spans)| InlineLink::new(target, spans))
            .collect();
        RichText::from_parts(Text::from(lines), links)
    }

    /// Appends one span or merges it with the matching previous span.
    ///
    /// # Arguments
    ///
    /// * `content` — Non-empty text for the span.
    /// * `style` — Ratatui style applied to the span.
    /// * `link` — Optional source-order link index owning the span.
    fn push_span(&mut self, content: &str, style: Style, link: Option<usize>) {
        let line = self
            .lines
            .last_mut()
            .expect("inline text always retains one logical line");
        if let Some(span) = line
            .last_mut()
            .filter(|span| span.span.style == style && span.link == link)
        {
            span.span.content.to_mut().push_str(content);
        } else {
            line.push(ParsedInlineSpan {
                span: Span::styled(content.to_owned(), style),
                link,
            });
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
/// * `link_base` — Directory used to resolve relative link targets.
///
/// # Returns
///
/// A [`Vec`] containing converted semantic views with empty paragraphs between
/// blocks in event order.
fn parse_blocks<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    end: Option<TagEnd>,
    options: MarkdownOptions,
    context: &mut MarkdownParseContext<'_>,
) -> Vec<View> {
    let mut views = Vec::new();
    let mut inline = InlineText::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(paragraph(parse_inline(events, TagEnd::Paragraph, context)));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                flush_inline_paragraph(&mut inline, &mut views);
                let content = parse_inline(events, TagEnd::Heading(level), context);
                let view = if context.source_path.is_some() {
                    let slug = context.heading_slug(&content);
                    heading(level, content).with_id(slug)
                } else {
                    heading(level, content)
                };
                views.push(view);
            }
            Event::Start(Tag::List(start)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_list(events, start, options, context));
            }
            Event::Start(Tag::Table(alignments)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_table(events, &alignments, context));
            }
            Event::Start(Tag::BlockQuote(kind)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(block_quote(parse_blocks(
                    events,
                    Some(TagEnd::BlockQuote(kind)),
                    options,
                    context,
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
            event => parse_inline_event(events, event, Style::new(), None, context, &mut inline),
        }
    }

    flush_inline_paragraph(&mut inline, &mut views);
    separate_blocks(views)
}

/// Inserts one empty terminal row between Markdown blocks.
///
/// CommonMark blank lines delimit blocks without producing parser events. An
/// empty semantic paragraph restores that document spacing while keeping the
/// surrounding column's existing measurement and scrolling behavior.
///
/// # Arguments
///
/// * `blocks` — Parsed Markdown blocks in source order.
///
/// # Returns
///
/// A [`Vec`] containing the blocks separated by empty paragraphs.
fn separate_blocks(blocks: Vec<View>) -> Vec<View> {
    let mut separated = Vec::with_capacity(blocks.len().saturating_mul(2).saturating_sub(1));

    for block in blocks {
        if !separated.is_empty() {
            separated.push(paragraph(""));
        }
        separated.push(block);
    }

    separated
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
                push_footnote_reference(&mut content, &label, Style::new(), None);
            }
            Event::SoftBreak | Event::HardBreak => content.push_break(),
            Event::TaskListMarker(checked) => {
                push_task_list_marker(&mut content, checked, Style::new(), None);
            }
            Event::Start(_) | Event::End(_) | Event::Rule => {}
        }
    }

    paragraph(content.into_rich_text())
}

/// Converts accumulated direct inline content into a semantic paragraph.
///
/// # Arguments
///
/// * `inline` — Pending styled inline text.
/// * `views` — Destination block sequence for the resulting paragraph.
fn flush_inline_paragraph(inline: &mut InlineText, views: &mut Vec<View>) {
    if inline.has_content() {
        views.push(paragraph(std::mem::take(inline).into_rich_text()));
    }
}

/// Collects styled inline content through a matching closing tag.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned after an opening tag.
/// * `end` — Closing tag that terminates the inline content.
/// * `context` — File and link-resolution context for this document.
///
/// # Returns
///
/// An owned [`RichText`] containing styled spans, links, and retained breaks.
fn parse_inline<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    end: TagEnd,
    context: &MarkdownParseContext<'_>,
) -> RichText {
    let mut content = InlineText::new();
    parse_inline_events(events, end, Style::new(), None, context, &mut content);
    content.into_rich_text()
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
/// * `active_link` — Optional source-order link index owning nested text.
/// * `context` — File and link-resolution context for this document.
/// * `content` — Destination rich-text accumulator.
fn parse_inline_events<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    end: TagEnd,
    style: Style,
    active_link: Option<usize>,
    context: &MarkdownParseContext<'_>,
    content: &mut InlineText,
) {
    while let Some(event) = events.next() {
        if event == Event::End(end) {
            break;
        }
        parse_inline_event(events, event, style, active_link, context, content);
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
/// * `active_link` — Optional source-order link index owning the event.
/// * `context` — File and link-resolution context for this document.
/// * `content` — Destination rich-text accumulator.
fn parse_inline_event<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    event: Event<'a>,
    style: Style,
    active_link: Option<usize>,
    context: &MarkdownParseContext<'_>,
    content: &mut InlineText,
) {
    match event {
        Event::Text(text)
        | Event::InlineMath(text)
        | Event::DisplayMath(text)
        | Event::Html(text)
        | Event::InlineHtml(text) => content.push_text_for_link(&text, style, active_link),
        Event::Code(text) => {
            content.push_text_for_link(&text, style.add_modifier(Modifier::REVERSED), active_link);
        }
        Event::SoftBreak | Event::HardBreak => content.push_break(),
        Event::Start(Tag::Emphasis) => parse_inline_events(
            events,
            TagEnd::Emphasis,
            style.add_modifier(Modifier::ITALIC),
            active_link,
            context,
            content,
        ),
        Event::Start(Tag::Strong) => parse_inline_events(
            events,
            TagEnd::Strong,
            style.add_modifier(Modifier::BOLD),
            active_link,
            context,
            content,
        ),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            ..
        }) => {
            parse_link(events, link_type, &dest_url, style, context, content);
        }
        Event::Start(Tag::Image { dest_url, .. }) => {
            parse_image(events, &dest_url, style, active_link, context, content);
        }
        Event::Start(Tag::CodeBlock(_)) => skip_until(events, TagEnd::CodeBlock),
        Event::Start(Tag::HtmlBlock) => skip_until(events, TagEnd::HtmlBlock),
        Event::FootnoteReference(label) => {
            push_footnote_reference(content, &label, style, active_link);
        }
        Event::TaskListMarker(checked) => {
            push_task_list_marker(content, checked, style, active_link);
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
/// * `active_link` — Optional outer link index owning the fallback text.
/// * `context` — File and link-resolution context for nested image content.
/// * `content` — Destination rich-text accumulator.
fn parse_image<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    destination: &str,
    style: Style,
    active_link: Option<usize>,
    context: &MarkdownParseContext<'_>,
    content: &mut InlineText,
) {
    let mut alt = InlineText::new();
    parse_inline_events(events, TagEnd::Image, style, None, context, &mut alt);
    let alt = alt.plain_text();
    let fallback = match (alt.is_empty(), destination.is_empty()) {
        (false, false) => format!("Image: {alt} ({destination})"),
        (false, true) => format!("Image: {alt}"),
        (true, false) => format!("Image: {destination}"),
        (true, true) => "Image".to_owned(),
    };
    content.push_text_for_link(&fallback, style, active_link);
}

/// Appends a readable footnote reference when such an event is enabled.
///
/// # Arguments
///
/// * `content` — Destination rich-text accumulator.
/// * `label` — Parsed footnote label.
/// * `style` — Span style inherited from the surrounding inline scope.
/// * `active_link` — Optional link index owning the reference text.
fn push_footnote_reference(
    content: &mut InlineText,
    label: &str,
    style: Style,
    active_link: Option<usize>,
) {
    content.push_text_for_link(&format!("[^{label}]"), style, active_link);
}

/// Appends a readable checkbox marker when task-list events are enabled.
///
/// # Arguments
///
/// * `content` — Destination rich-text accumulator.
/// * `checked` — Whether the parsed task marker is checked.
/// * `style` — Span style inherited from the surrounding inline scope.
/// * `active_link` — Optional link index owning the task marker.
fn push_task_list_marker(
    content: &mut InlineText,
    checked: bool,
    style: Style,
    active_link: Option<usize>,
) {
    content.push_text_for_link(if checked { "[x] " } else { "[ ] " }, style, active_link);
}

/// Parses a Markdown link into a focusable label range.
///
/// Link labels retain surrounding emphasis or strong modifiers. The target is
/// stored as metadata rather than appended to non-empty visible labels.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside a link.
/// * `link_type` — Parser classification used to retain `mailto:` activation.
/// * `destination` — Parsed link destination.
/// * `style` — Span style inherited from the surrounding inline scope.
/// * `context` — File and link-resolution context for this document.
/// * `content` — Destination rich-text accumulator.
fn parse_link<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    link_type: LinkType,
    destination: &str,
    style: Style,
    context: &MarkdownParseContext<'_>,
    content: &mut InlineText,
) {
    let target = context.link_target(link_type, destination);
    let link = content.push_link(target);
    parse_inline_events(events, TagEnd::Link, style, Some(link), context, content);
    if !content.link_has_text(link) && !destination.is_empty() {
        content.push_text_for_link(destination, style, Some(link));
    }
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
fn heading(level: HeadingLevel, content: RichText) -> View {
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
/// * `context` — File/link context and heading state for nested content.
///
/// # Returns
///
/// A semantic ordered or unordered list retaining item order and nesting.
fn parse_list<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    start: Option<u64>,
    options: MarkdownOptions,
    context: &mut MarkdownParseContext<'_>,
) -> View {
    let mut items = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::Item) => {
                items.push(list_item(parse_blocks(
                    events,
                    Some(TagEnd::Item),
                    options,
                    context,
                )));
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
/// * `context` — File and link-resolution context for the table.
///
/// # Returns
///
/// A semantic table containing one header section and one body section.
fn parse_table<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    alignments: &[Alignment],
    context: &MarkdownParseContext<'_>,
) -> View {
    let mut header_rows = Vec::new();
    let mut body_rows = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::TableHead) => {
                header_rows.push(parse_table_cells(
                    events,
                    alignments,
                    TagEnd::TableHead,
                    context,
                ));
            }
            Event::Start(Tag::TableRow) => {
                body_rows.push(parse_table_cells(
                    events,
                    alignments,
                    TagEnd::TableRow,
                    context,
                ));
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
/// * `context` — File and link-resolution context for table cells.
///
/// # Returns
///
/// A semantic table-row view containing aligned cells.
fn parse_table_cells<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    alignments: &[Alignment],
    end: TagEnd,
    context: &MarkdownParseContext<'_>,
) -> View {
    let mut cells = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::TableCell) => {
                let alignment = alignment_at(alignments, cells.len());
                cells.push(
                    table_cell(parse_inline(events, TagEnd::TableCell, context))
                        .alignment(alignment),
                );
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
        io,
        path::PathBuf,
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

    /// Returns the state inside a file-backed Markdown boundary.
    ///
    /// # Arguments
    ///
    /// * `view` — View expected to have been built by a Markdown file reader.
    ///
    /// # Returns
    ///
    /// The boundary's retained [`MarkdownView`] state.
    fn markdown_file_state(view: &View) -> &MarkdownView {
        let View::Markdown { state } = view else {
            panic!("expected file-backed Markdown boundary, got {view:?}");
        };
        state
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
        let view = match view {
            View::Markdown { state } => state.document(),
            view => view,
        };
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
        let view = match view {
            View::Markdown { state } => state.document(),
            view => view,
        };
        let View::Column { metadata, .. } = view else {
            panic!("expected Markdown column, got {view:?}");
        };

        (metadata.scroll_offset(), metadata.max_scroll_offset())
    }

    /// Renders a view into fixed terminal rows for fallback assertions.
    ///
    /// # Arguments
    ///
    /// * `view` — View tree to render.
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
    fn rendered_view_lines(view: &View, width: u16, height: u16) -> Result<Vec<String>> {
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
        rendered_view_lines(&markdown(source), width, height)
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

        let default = markdown_file(&fixture_path);
        let View::Markdown { state } = &default else {
            panic!("expected file-backed Markdown boundary");
        };
        assert_eq!(state.current_path(), fixture_path);
        assert_eq!(state.document(), &markdown(source));

        let options = MarkdownOptions::default()
            .syntax_theme(SyntaxTheme::Light)
            .line_numbers(true);
        let configured = markdown_file_with_options(&fixture_path, options);
        assert_eq!(
            parsed_code_block_options(&configured),
            (true, SyntaxTheme::Light)
        );

        fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
    }

    /// Verifies Markdown file failures become path-aware semantic fallbacks.
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
    /// - Missing paths produce a paragraph containing the path and not-found error.
    /// - Directory paths produce a paragraph containing their platform I/O failure.
    /// - Invalid UTF-8 produces a paragraph containing the path and decoding error.
    /// - Every failure remains inside a scrollable document column.
    /// - The missing-file fallback renders visibly without propagating an error.
    /// - The fixture directory is removed after verification.
    #[test]
    fn markdown_file_failures_render_path_aware_fallbacks() {
        let fixture_dir = markdown_fixture_dir("errors");
        let directory_path = fixture_dir.join("directory.md");
        let invalid_utf8_path = fixture_dir.join("invalid-utf8.md");
        let missing_path = fixture_dir.join("missing.md");
        fs::create_dir_all(&directory_path).expect("directory fixture should be created");
        fs::write(&invalid_utf8_path, [0xff, 0xfe])
            .expect("invalid UTF-8 fixture should be written");

        let expected_fallback = |path: &Path, error: &io::Error| {
            column([paragraph(format!(
                "failed to read Markdown file `{}`: {error}",
                path.display()
            ))])
        };

        let missing_error =
            fs::read_to_string(&missing_path).expect_err("missing fixture should fail to read");
        assert_eq!(missing_error.kind(), io::ErrorKind::NotFound);
        let missing = markdown_file(&missing_path);
        let View::Markdown { state } = &missing else {
            panic!("expected file-backed Markdown boundary");
        };
        assert_eq!(
            state.document(),
            &expected_fallback(&missing_path, &missing_error)
        );
        let rendered = rendered_view_lines(&missing, 120, 2)
            .expect("missing-file fallback should render without failure")
            .concat();
        assert!(rendered.contains("failed to read Markdown file"));
        assert!(rendered.contains("missing.md"));

        let directory_error =
            fs::read_to_string(&directory_path).expect_err("directory fixture should fail to read");
        assert_ne!(directory_error.kind(), io::ErrorKind::NotFound);
        let directory = markdown_file(&directory_path);
        let View::Markdown { state } = &directory else {
            panic!("expected file-backed Markdown boundary");
        };
        assert_eq!(
            state.document(),
            &expected_fallback(&directory_path, &directory_error)
        );

        let invalid_utf8_error = fs::read_to_string(&invalid_utf8_path)
            .expect_err("invalid UTF-8 fixture should fail to read");
        assert_eq!(invalid_utf8_error.kind(), io::ErrorKind::InvalidData);
        let invalid = markdown_file_with_options(&invalid_utf8_path, MarkdownOptions::default());
        let View::Markdown { state } = &invalid else {
            panic!("expected file-backed Markdown boundary");
        };
        assert_eq!(
            state.document(),
            &expected_fallback(&invalid_utf8_path, &invalid_utf8_error)
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
    /// - Empty separator paragraphs retain one terminal row between headings.
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
            column(separate_blocks(vec![
                h1("One"),
                h2("Two"),
                h3("Three"),
                h4("Four"),
                h5("Five"),
                h6("Six"),
            ])),
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

    /// Verifies Markdown blocks retain a blank terminal row between them.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// One
    ///
    /// Two
    /// ```
    ///
    /// # Assertions
    ///
    /// - Parsing inserts one empty semantic paragraph between the blocks.
    /// - Rendering retains the empty row between the visible paragraph rows.
    #[test]
    fn markdown_separates_blocks_with_blank_terminal_rows() -> Result<()> {
        let source = "One\n\nTwo\n";

        assert_eq!(
            markdown(source),
            column([paragraph("One"), paragraph(""), paragraph("Two")]),
        );
        assert_eq!(
            rendered_markdown_lines(source, 8, 3)?,
            ["One     ", "        ", "Two     "],
        );

        Ok(())
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
    /// - Empty separator paragraphs retain one terminal row between code blocks.
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
            column(separate_blocks(vec![
                code_block("fn main() {}\n").language("rust"),
                code_block("let value = true;\n").language("rs"),
                code_block("plain\n").language("unknown-language"),
            ])),
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
    /// - An empty separator paragraph retains one terminal row between blocks.
    #[test]
    fn markdown_maps_empty_and_indented_code_blocks() {
        let source = "```\n```\n\n    plain 界\n";

        assert_eq!(
            markdown(source),
            column(separate_blocks(vec![
                code_block(""),
                code_block("plain 界\n"),
            ])),
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

    /// Verifies Markdown links retain focusable targets without exposing destinations.
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
    /// - Four non-empty URL and email targets participate in focus traversal.
    /// - Link labels retain nested emphasis in visible rich text.
    /// - Descriptive labels do not append their hidden destination.
    /// - Email labels do not expose the `mailto:` activation scheme.
    /// - Empty destinations remain visible but inactive.
    #[test]
    fn markdown_retains_actionable_links_with_label_only_text() {
        let source = concat!(
            "Read [the *guide*](https://example.com/guide), ",
            "[https://example.com](https://example.com), ",
            "<https://example.org>, and <reader@example.com>, plus [empty]().\n",
        );
        let document = markdown(source);
        assert_eq!(document.__focusable_count(), 4);
        let View::Column { children, .. } = &document else {
            panic!("expected Markdown column, got {document:?}");
        };
        let [View::Paragraph { content, .. }] = children.as_slice() else {
            panic!("expected one Markdown paragraph, got {children:?}");
        };
        assert_eq!(
            content.to_string(),
            concat!(
                "Read the guide, https://example.com, https://example.org, and ",
                "reader@example.com, plus empty."
            )
        );
        assert!(
            content.text().lines[0].spans[2]
                .style
                .add_modifier
                .contains(Modifier::ITALIC)
        );
    }

    /// Verifies links remain focusable across semantic Markdown containers.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// # [Heading](https://example.com/heading)
    /// - [List](https://example.com/list)
    /// | [Head](https://example.com/head) |
    /// | --- |
    /// | [Cell](https://example.com/cell) |
    /// ```
    ///
    /// # Assertions
    ///
    /// - Heading links participate in focus traversal.
    /// - Links nested in list paragraphs participate in focus traversal.
    /// - Header and body table-cell links participate in focus traversal.
    #[test]
    fn markdown_links_survive_heading_list_and_table_conversion() {
        let source = concat!(
            "# [Heading](https://example.com/heading)\n\n",
            "- [List](https://example.com/list)\n\n",
            "| [Head](https://example.com/head) |\n",
            "| --- |\n",
            "| [Cell](https://example.com/cell) |\n",
        );

        assert_eq!(markdown(source).__focusable_count(), 4);
    }

    /// Verifies Markdown links participate in focus and default link styling.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// [Guide](https://example.com) and [Section](#part)
    /// Tab
    /// ```
    ///
    /// # Assertions
    ///
    /// - Only the absolute URL contributes to focus traversal.
    /// - The initial URL label is underlined without reverse video.
    /// - Tab focuses the URL and adds the default focused reverse modifier.
    #[test]
    fn markdown_links_render_and_receive_focus() -> Result<()> {
        let mut document = markdown("[Guide](https://example.com) and [Section](#part)");
        assert_eq!(document.__focusable_count(), 1);
        let mut terminal = Terminal::new(TestBackend::new(24, 1))?;
        let mut initial_render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            initial_render_result = document.render(&mut ctx);
        })?;
        initial_render_result?;
        let initial = terminal.backend().buffer().content()[0].modifier;
        assert!(initial.contains(Modifier::UNDERLINED));
        assert!(!initial.contains(Modifier::REVERSED));

        assert_eq!(
            document.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?,
            crate::KeyControl::Handled
        );
        let mut focused_render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            focused_render_result = document.render(&mut ctx);
        })?;
        focused_render_result?;
        let focused = terminal.backend().buffer().content()[0].modifier;
        assert!(focused.contains(Modifier::UNDERLINED));
        assert!(focused.contains(Modifier::REVERSED));
        Ok(())
    }

    /// Verifies focusing a wrapped Markdown link scrolls its exact text rows into view.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// one two three four five six [Guide](https://example.com)
    /// terminal size = 10x2
    /// Tab, render
    /// ```
    ///
    /// # Assertions
    ///
    /// - Focus causes the overflowing document column to scroll downward.
    /// - The focused link label becomes visible in the terminal buffer.
    ///
    /// # Why
    ///
    /// Treating an entire wrapped paragraph as the focused span would keep its
    /// first row visible while leaving a link near the end offscreen.
    #[test]
    fn focused_wrapped_markdown_link_scrolls_into_view() -> Result<()> {
        let mut document = markdown("one two three four five six [Guide](https://example.com)");
        assert_eq!(
            document.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?,
            crate::KeyControl::Handled
        );

        let mut terminal = Terminal::new(TestBackend::new(10, 2))?;
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            render_result = document.render(&mut ctx);
        })?;
        render_result?;

        let (scroll_offset, _) = markdown_scroll_state(&document);
        assert!(scroll_offset > 0);
        assert!(
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .any(|cell| cell.symbol() == "G")
        );
        Ok(())
    }

    /// Verifies focused Markdown links survive regenerated-view reconciliation.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// previous = markdown("[Guide](https://example.com)") + Tab
    /// next = markdown("[Updated](https://example.com)")
    /// reconcile(next, previous)
    /// ```
    ///
    /// # Assertions
    ///
    /// - A regenerated inline link with the same target retains focus.
    /// - Its updated visible label remains intact.
    #[test]
    fn markdown_link_focus_survives_reconciliation() -> Result<()> {
        let mut previous = markdown("[Guide](https://example.com)");
        previous.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
        let mut next = markdown("[Updated](https://example.com)");

        crate::__private::__reconcile_view(&mut next, &previous);
        let mut index = 0;
        assert_eq!(next.__focused_index_inner(&mut index), Some(0));
        let View::Column { children, .. } = &next else {
            panic!("expected Markdown column");
        };
        let [View::Paragraph { content, .. }] = children.as_slice() else {
            panic!("expected Markdown paragraph");
        };
        assert_eq!(content.to_string(), "Updated");
        Ok(())
    }

    /// Verifies file-backed Markdown resolves relative links from its own directory.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// /tmp/fixture/reader.md contains [Missing](nested/missing.md)
    /// Tab, Enter
    /// ```
    ///
    /// # Assertions
    ///
    /// - The relative link is focusable.
    /// - Activation opens the missing target as an in-app error page.
    /// - The current path is resolved below the Markdown file's directory.
    /// - Back history retains the source page.
    #[test]
    fn markdown_file_links_resolve_from_source_directory() {
        let fixture_dir = markdown_fixture_dir("link-base");
        fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
        let markdown_path = fixture_dir.join("reader.md");
        fs::write(&markdown_path, "[Missing](nested/missing.md)")
            .expect("Markdown fixture should be written");

        let mut document = markdown_file(&markdown_path);
        assert_eq!(document.__focusable_count(), 1);
        document
            .handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
            .expect("link focus should succeed");
        assert_eq!(
            document
                .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
                .expect("missing relative Markdown should open its error page"),
            crate::KeyControl::Handled,
        );
        let View::Markdown { state } = &document else {
            panic!("expected file-backed Markdown boundary");
        };
        assert_eq!(state.current_path(), fixture_dir.join("nested/missing.md"));
        assert!(state.can_go_back());
        assert!(!state.can_go_forward());

        fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
    }

    /// Verifies in-app targets are classified only for file-backed Markdown.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// [Guide](nested/Guide.MD#part)
    /// [Long](other.MarkDown)
    /// [Remote](https://example.com/remote.md)
    /// [Section](#part)
    /// [Bare](#)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Markdown extensions are recognized case-insensitively and resolved
    ///   from the source directory.
    /// - Non-empty fragments target the current file.
    /// - Remote Markdown URLs remain external URLs and bare hashes stay inert.
    /// - In-memory Markdown retains its existing path/fragment targets.
    #[test]
    fn markdown_file_classifies_in_app_targets_at_its_boundary() {
        let fixture_dir = markdown_fixture_dir("target-classification");
        let markdown_path = fixture_dir.join("reader.md");
        let source = concat!(
            "[Guide](nested/Guide.MD#part) ",
            "[Long](other.MarkDown) ",
            "[Remote](https://example.com/remote.md) ",
            "[Section](#part) ",
            "[Bare](#)",
        );
        fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
        fs::write(&markdown_path, source).expect("Markdown fixture should be written");

        let file = markdown_file(&markdown_path);
        let View::Column { children, .. } = markdown_file_state(&file).document() else {
            panic!("expected parsed Markdown column");
        };
        let [View::Paragraph { content, .. }] = children.as_slice() else {
            panic!("expected one linked paragraph");
        };
        let targets = content
            .links()
            .iter()
            .map(|link| link.target().clone())
            .collect::<Vec<_>>();
        assert_eq!(
            targets,
            vec![
                LinkTarget::Markdown {
                    path: fixture_dir.join("nested/Guide.MD"),
                    fragment: Some("part".to_owned()),
                },
                LinkTarget::Markdown {
                    path: fixture_dir.join("other.MarkDown"),
                    fragment: None,
                },
                LinkTarget::Url("https://example.com/remote.md".to_owned()),
                LinkTarget::Markdown {
                    path: markdown_path.clone(),
                    fragment: Some("part".to_owned()),
                },
                LinkTarget::Fragment("#".to_owned()),
            ]
        );

        let View::Column { children, .. } = markdown("[Guide](guide.md) [Section](#part)") else {
            panic!("expected in-memory Markdown column");
        };
        let [View::Paragraph { content, .. }] = children.as_slice() else {
            panic!("expected one in-memory paragraph");
        };
        assert_eq!(
            content.links()[0].target(),
            &LinkTarget::Path(
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("guide.md")
            )
        );
        assert_eq!(content.links()[1].target(), &LinkTarget::from("#part"));

        fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
    }

    /// Verifies back/forward navigation swaps complete cached Markdown pages.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// root.md -> b.md -> Shift+H -> Shift+L
    /// root.md -> b.md -> Shift+H -> c.md
    /// ```
    ///
    /// # Assertions
    ///
    /// - Shift+H and Shift+L traverse cached page history.
    /// - Returning to a page restores its exact focus and scroll offset.
    /// - A new navigation after going back clears forward history.
    /// - Reconciliation preserves state for the same root/options and resets it
    ///   when options change.
    #[test]
    fn markdown_file_history_restores_cached_state_and_reconciles() -> Result<()> {
        let fixture_dir = markdown_fixture_dir("history");
        let root_path = fixture_dir.join("root.md");
        let b_path = fixture_dir.join("b.md");
        let c_path = fixture_dir.join("c.md");
        let mut source = (0..12)
            .map(|index| format!("Paragraph {index}.\n\n"))
            .collect::<String>();
        source.push_str("[B](b.md) [C](c.md)\n");
        fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
        fs::write(&root_path, source).expect("root Markdown fixture should be written");
        fs::write(&b_path, "# Page B\n").expect("B fixture should be written");
        fs::write(&c_path, "# Page C\n").expect("C fixture should be written");

        let mut document = markdown_file(&root_path);
        rendered_view_lines(&document, 30, 5)?;
        document.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
        rendered_view_lines(&document, 30, 5)?;
        let root_scroll = markdown_scroll_state(&document);
        assert!(root_scroll.0 > 0);

        assert_eq!(
            document.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
            crate::KeyControl::Handled
        );
        assert_eq!(markdown_file_state(&document).current_path(), b_path);
        assert!(markdown_file_state(&document).can_go_back());

        assert_eq!(
            document.handle_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT,))?,
            crate::KeyControl::Handled
        );
        assert_eq!(markdown_file_state(&document).current_path(), root_path);
        assert!(markdown_file_state(&document).can_go_forward());
        assert_eq!(markdown_scroll_state(&document), root_scroll);
        let mut focused_index = 0;
        assert_eq!(document.__focused_index_inner(&mut focused_index), Some(0));

        assert_eq!(
            document.handle_key_event(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT,))?,
            crate::KeyControl::Handled
        );
        assert_eq!(markdown_file_state(&document).current_path(), b_path);

        document.handle_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT))?;
        document.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
        document.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;
        assert_eq!(markdown_file_state(&document).current_path(), c_path);
        assert!(!markdown_file_state(&document).can_go_forward());

        let previous = document.clone();
        let mut reconciled = markdown_file(&root_path);
        crate::__private::__reconcile_view(&mut reconciled, &previous);
        assert_eq!(markdown_file_state(&reconciled).current_path(), c_path);
        assert!(markdown_file_state(&reconciled).can_go_back());

        let mut reset =
            markdown_file_with_options(&root_path, MarkdownOptions::default().line_numbers(true));
        crate::__private::__reconcile_view(&mut reset, &previous);
        assert_eq!(markdown_file_state(&reset).current_path(), root_path);
        assert!(!markdown_file_state(&reset).can_go_back());

        fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
        Ok(())
    }

    /// Verifies heading fragments load in-app and align their heading at top.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// [Duplicate](target.md#repeat-heading-1)
    /// [Same file](#caf%C3%A9)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Duplicate GitHub-style slugs receive numeric suffixes.
    /// - Percent-encoded Unicode fragments are decoded case-insensitively.
    /// - Cross-file and same-file fragments create history entries and scroll
    ///   the matching heading to the top.
    /// - Missing anchors leave the loaded page at its default top position.
    #[test]
    fn markdown_file_fragments_use_heading_anchors() -> Result<()> {
        let fixture_dir = markdown_fixture_dir("anchors");
        let root_path = fixture_dir.join("root.md");
        let target_path = fixture_dir.join("target.md");
        fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
        fs::write(
            &root_path,
            "[Duplicate](target.md#repeat-heading-1) [Missing](target.md#missing)\n\n\
             Filler.\n\nFiller.\n\nFiller.\n\nFiller.\n\n## Café\n",
        )
        .expect("root Markdown fixture should be written");
        let target = format!(
            "{}## Repeat Heading\n\nBetween.\n\n## Repeat Heading\n",
            (0..12)
                .map(|index| format!("Target paragraph {index}.\n\n"))
                .collect::<String>()
        );
        fs::write(&target_path, target).expect("target Markdown fixture should be written");

        let mut document = markdown_file(&root_path);
        document.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
        document.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;
        let lines = rendered_view_lines(&document, 32, 5)?;
        assert_eq!(markdown_file_state(&document).current_path(), target_path);
        assert!(lines[0].starts_with("## Repeat Heading"));
        assert!(markdown_scroll_state(&document).0 > 0);

        document.handle_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT))?;
        document.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
        document.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;
        let missing_lines = rendered_view_lines(&document, 32, 5)?;
        assert_eq!(markdown_file_state(&document).current_path(), target_path);
        assert_eq!(markdown_scroll_state(&document).0, 0);
        assert!(missing_lines[0].starts_with("Target paragraph 0."));

        fs::write(
            &root_path,
            "[Café](#caf%C3%A9)\n\nFiller.\n\n## Café\n\nAfter.\n\nAfter.\n",
        )
        .expect("same-file fragment fixture should be written");
        let mut same_file = markdown_file(&root_path);
        same_file.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
        same_file.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;
        let same_file_lines = rendered_view_lines(&same_file, 24, 2)?;
        assert_eq!(markdown_file_state(&same_file).current_path(), root_path);
        assert!(markdown_file_state(&same_file).can_go_back());
        assert!(same_file_lines[0].starts_with("## Café"));

        fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
        Ok(())
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
    /// - Empty separator paragraphs retain block spacing inside loose items.
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
                list_item(separate_blocks(vec![
                    paragraph("First"),
                    paragraph("Second paragraph."),
                    unordered_list([
                        list_item(separate_blocks(vec![
                            paragraph("Nested bullet"),
                            ordered_list([list_item([paragraph("Nested number")])]).start(7),
                        ])),
                        list_item([]),
                    ]),
                ])),
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
    /// - The blank row between quote blocks retains the outer quote border.
    #[test]
    fn markdown_renders_nested_blockquotes_with_wrapped_prefixes() -> Result<()> {
        let source = "> Alpha beta gamma\n>\n> > Inner\n";

        assert_eq!(
            markdown(source),
            column([block_quote(vec![
                paragraph("Alpha beta gamma"),
                paragraph(""),
                block_quote(vec![paragraph("Inner")]),
            ])]),
        );

        let lines = rendered_markdown_lines(source, 12, 4)?;
        assert!(lines[0].starts_with("│ Alpha beta"));
        assert!(lines[1].starts_with("│ gamma"));
        assert_eq!(lines[2].trim_end(), "│");
        assert!(lines[3].starts_with("│ │ Inner"));

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
    /// - Empty separator paragraphs retain one terminal row between images.
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
            column(separate_blocks(vec![
                paragraph("Image: diagram (https://example.com/diagram.png)"),
                paragraph("Image: local.png"),
                paragraph("Image: caption"),
                paragraph("Image"),
            ])),
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
    /// - Empty separator paragraphs retain one terminal row between blocks.
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
            column(separate_blocks(vec![
                paragraph("Before <kbd>&</kbd> after."),
                paragraph(Text::from(vec![
                    Line::raw("<section>"),
                    Line::raw("block &amp;"),
                    Line::raw("</section>"),
                    Line::default(),
                ])),
                paragraph("Fish & Chips ©"),
            ])),
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
    /// - Empty separator paragraphs retain one terminal row between blocks.
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
        let mut context = MarkdownParseContext::new(Path::new("."), None);

        assert_eq!(
            column(parse_blocks(
                &mut parser,
                None,
                MarkdownOptions::default(),
                &mut context,
            )),
            column(separate_blocks(vec![
                unordered_list([list_item([paragraph("[x] done and x + y[^note]")])]),
                paragraph("z"),
                paragraph("Detail"),
            ])),
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
    /// - Empty separator paragraphs retain one terminal row between fallbacks.
    #[test]
    fn markdown_preserves_fallback_source_order() {
        let source = "# Start\n\n---\n\n![middle](middle.png)\n\n<end>\n";

        assert_eq!(
            markdown(source),
            column(separate_blocks(vec![
                h1("Start"),
                thematic_break(),
                paragraph("Image: middle (middle.png)"),
                paragraph(Text::from(vec![Line::raw("<end>"), Line::default()])),
            ])),
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
    /// - Empty separator paragraphs retain one terminal row between blocks.
    #[test]
    fn markdown_preserves_source_order() {
        let source = "# 開始\n\nBefore.\n\n- 中\n\n## 終了\n";

        assert_eq!(
            markdown(source),
            column(separate_blocks(vec![
                h1("開始"),
                paragraph("Before."),
                unordered_list([list_item([paragraph("中")])]),
                h2("終了"),
            ])),
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
