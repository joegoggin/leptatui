//! Link targets and linked rich-text storage.
//!
//! This module classifies URL, filesystem, and fragment targets and retains
//! focusable link ranges inside semantic rich text. Standalone and embedded
//! links share the same target resolution and system-opening behavior.

use std::{
    borrow::Cow,
    collections::VecDeque,
    ffi::OsStr,
    fmt, io,
    ops::Deref,
    path::{Path, PathBuf},
};

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    TuiStyle,
    app::{AppControl, Error, Result},
    component::{FocusedControl, RenderCtx},
};

use super::{
    CellAlignment,
    core::{
        metadata::{StyleMetadata, ViewType},
        render::{VerticalSpan, resolve_style},
    },
};

/// Destination retained by a standalone or embedded link.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkTarget {
    /// Absolute URI passed to the operating system's configured handler.
    Url(String),
    /// Absolute or relative filesystem path passed to its configured application.
    Path(PathBuf),
    /// Markdown file eligible for in-app file-backed navigation.
    Markdown {
        /// Absolute or relative Markdown file path.
        path: PathBuf,
        /// Optional heading fragment to reveal after loading.
        fragment: Option<String>,
    },
    /// Empty or in-document fragment target retained without activation.
    Fragment(String),
}

impl LinkTarget {
    /// Returns whether this target can be activated.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether this is an external or in-app target.
    pub const fn is_actionable(&self) -> bool {
        matches!(self, Self::Url(_) | Self::Path(_) | Self::Markdown { .. })
    }

    /// Resolves a relative filesystem target against a base directory.
    ///
    /// URL, absolute path, and fragment targets remain unchanged. Relative
    /// Markdown paths retain their optional fragment while being resolved.
    ///
    /// # Arguments
    ///
    /// * `base` — Directory used to resolve relative filesystem paths.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget`] containing an absolute or base-relative path.
    pub fn resolve_against(self, base: impl AsRef<Path>) -> Self {
        match self {
            Self::Path(path) if path.is_relative() => Self::Path(base.as_ref().join(path)),
            Self::Markdown { path, fragment } if path.is_relative() => Self::Markdown {
                path: base.as_ref().join(path),
                fragment,
            },
            target => target,
        }
    }

    /// Returns the user-facing destination text.
    ///
    /// # Returns
    ///
    /// A [`String`] containing the URI, path, or fragment.
    pub fn display(&self) -> String {
        match self {
            Self::Url(url) | Self::Fragment(url) => url.clone(),
            Self::Path(path) => path.display().to_string(),
            Self::Markdown { path, fragment } => fragment.as_ref().map_or_else(
                || path.display().to_string(),
                |fragment| format!("{}#{fragment}", path.display()),
            ),
        }
    }

    /// Returns the operating-system argument for an actionable target.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the URL or path as an [`OsStr`].
    fn as_os_str(&self) -> Option<&OsStr> {
        match self {
            Self::Url(url) => Some(OsStr::new(url)),
            Self::Path(path) | Self::Markdown { path, .. } => Some(path.as_os_str()),
            Self::Fragment(_) => None,
        }
    }
}

impl From<String> for LinkTarget {
    /// Classifies owned destination text as a URL, path, or fragment.
    ///
    /// # Arguments
    ///
    /// * `value` — Destination text to classify.
    ///
    /// # Returns
    ///
    /// A classified [`LinkTarget`].
    fn from(value: String) -> Self {
        if value.is_empty() || value.starts_with('#') {
            Self::Fragment(value)
        } else if !has_windows_drive_prefix(&value) && has_uri_scheme(&value) {
            Self::Url(value)
        } else {
            Self::Path(PathBuf::from(value))
        }
    }
}

impl From<&str> for LinkTarget {
    /// Classifies borrowed destination text as a URL, path, or fragment.
    ///
    /// # Arguments
    ///
    /// * `value` — Destination text to classify and copy.
    ///
    /// # Returns
    ///
    /// A classified [`LinkTarget`].
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

impl From<PathBuf> for LinkTarget {
    /// Converts an explicit path buffer into a filesystem link target.
    ///
    /// # Arguments
    ///
    /// * `value` — Filesystem path to open.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget::Path`] containing `value`.
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<&Path> for LinkTarget {
    /// Converts an explicit borrowed path into a filesystem link target.
    ///
    /// # Arguments
    ///
    /// * `value` — Filesystem path to copy.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget::Path`] containing the copied path.
    fn from(value: &Path) -> Self {
        Self::Path(value.to_path_buf())
    }
}

impl From<&PathBuf> for LinkTarget {
    /// Converts an explicit borrowed path buffer into a filesystem link target.
    ///
    /// # Arguments
    ///
    /// * `value` — Filesystem path to copy.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget::Path`] containing the copied path.
    fn from(value: &PathBuf) -> Self {
        Self::Path(value.clone())
    }
}

/// Rich text with optional focusable link ranges.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RichText {
    /// Ratatui text retained for measurement and rendering.
    text: Text<'static>,
    /// Link metadata and span locations embedded in the text.
    links: Vec<InlineLink>,
}

impl RichText {
    /// Returns the underlying Ratatui text.
    ///
    /// # Returns
    ///
    /// A [`Text`] reference containing the visible rich text.
    pub const fn text(&self) -> &Text<'static> {
        &self.text
    }

    /// Creates linked rich text from parsed text and inline links.
    ///
    /// # Arguments
    ///
    /// * `text` — Visible Ratatui text.
    /// * `links` — Inline link metadata and span positions.
    ///
    /// # Returns
    ///
    /// A [`RichText`] containing the supplied parsed content.
    pub(crate) fn from_parts(text: Text<'static>, links: Vec<InlineLink>) -> Self {
        Self { text, links }
    }

    /// Returns the number of actionable links embedded in this text.
    pub(crate) fn focusable_count(&self) -> usize {
        self.links
            .iter()
            .filter(|link| link.target.is_actionable())
            .count()
    }

    /// Returns the focused embedded-link index during flattened traversal.
    pub(crate) fn focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        for link in &self.links {
            if !link.target.is_actionable() {
                continue;
            }
            let current = *index;
            *index = index.saturating_add(1);
            if link.metadata.is_focused() {
                return Some(current);
            }
        }
        None
    }

    /// Sets embedded-link focus during flattened traversal.
    pub(crate) fn set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        for link in &mut self.links {
            if !link.target.is_actionable() {
                link.metadata.set_focused(false);
                link.metadata.clear_scroll_into_view_request();
                continue;
            }
            let focused = *index == target;
            link.metadata.set_focused(focused);
            if focused {
                link.metadata.request_scroll_into_view();
            } else {
                link.metadata.clear_scroll_into_view_request();
            }
            *index = index.saturating_add(1);
        }
    }

    /// Opens the focused embedded link, if any.
    pub(crate) fn activate_focused_link(&self) -> Result<Option<AppControl>> {
        for link in &self.links {
            if link.metadata.is_focused() && link.target.is_actionable() {
                return open_link_target(&link.target).map(Some);
            }
        }
        Ok(None)
    }

    /// Returns the target of the focused actionable embedded link.
    pub(crate) fn focused_link_target(&self) -> Option<LinkTarget> {
        self.links
            .iter()
            .find(|link| link.metadata.is_focused() && link.target.is_actionable())
            .map(|link| link.target.clone())
    }

    /// Returns focused-control metadata for an embedded link.
    pub(crate) fn focused_control(&self) -> Option<FocusedControl> {
        self.links
            .iter()
            .any(|link| link.metadata.is_focused() && link.target.is_actionable())
            .then_some(FocusedControl::Link)
    }

    /// Returns whether a focused embedded link requested scrolling.
    pub(crate) fn focused_link_requested_scroll(&self) -> bool {
        self.links
            .iter()
            .any(|link| link.metadata.is_focused() && link.metadata.scroll_into_view_requested())
    }

    /// Returns the wrapped row span of the focused embedded link.
    pub(crate) fn focused_link_span(&self, width: u16) -> Option<VerticalSpan> {
        if width == 0 {
            return None;
        }

        let link = self.links.iter().position(|link| {
            link.metadata.is_focused() && link.metadata.scroll_into_view_requested()
        })?;
        let segments =
            linked_visual_segments(&self.text, &self.links, width, RichTextWrapMode::Word);
        let top = segments
            .iter()
            .filter(|segment| segment.link == link)
            .map(|segment| u32::from(segment.row))
            .min()?;
        let bottom = segments
            .iter()
            .filter(|segment| segment.link == link)
            .map(|segment| u32::from(segment.row).saturating_add(1))
            .max()?;

        Some(VerticalSpan { top, bottom })
    }

    /// Clears completed embedded-link scroll requests after rendering.
    pub(crate) fn clear_link_scroll_requests(&self) {
        for link in &self.links {
            link.metadata.clear_scroll_into_view_request();
        }
    }

    /// Clears last-rendered hit areas for embedded links.
    pub(crate) fn clear_link_hit_areas(&self) {
        for link in &self.links {
            link.metadata.clear_hit_areas();
        }
    }

    /// Records last-rendered hit areas for embedded links.
    ///
    /// # Arguments
    ///
    /// * `area` — Visible rich-text render area.
    /// * `width` — Width used to wrap the rich text.
    /// * `alignment` — Horizontal alignment applied to wrapped rows.
    /// * `wrap_mode` — Wrapping behavior used by the renderer.
    /// * `ctx` — Render context used to map local areas to terminal coordinates.
    pub(crate) fn record_link_hit_areas(
        &self,
        area: Rect,
        width: u16,
        alignment: CellAlignment,
        wrap_mode: RichTextWrapMode,
        ctx: &RenderCtx<'_, '_>,
    ) {
        self.clear_link_hit_areas();
        if width == 0 || area.height == 0 || self.links.is_empty() {
            return;
        }

        for segment in linked_visual_segments(&self.text, &self.links, width, wrap_mode) {
            if segment.row >= area.height {
                continue;
            }

            let line_offset = aligned_line_offset(segment.line_width, width, alignment);
            let hit_area = Rect {
                x: area
                    .x
                    .saturating_add(line_offset)
                    .saturating_add(segment.start),
                y: area.y.saturating_add(segment.row),
                width: segment.end.saturating_sub(segment.start),
                height: 1,
            };
            if let Some(hit_area) = ctx.map_hit_area(hit_area)
                && let Some(link) = self.links.get(segment.link)
            {
                link.metadata.push_hit_area(hit_area);
            }
        }
    }

    /// Returns the embedded-link index under a terminal position.
    pub(crate) fn focusable_index_at_position(
        &self,
        column: u16,
        row: u16,
        index: &mut usize,
    ) -> Option<usize> {
        for link in &self.links {
            if !link.target.is_actionable() {
                continue;
            }
            let current = *index;
            *index = index.saturating_add(1);
            if link.metadata.contains_hit_position(column, row) {
                return Some(current);
            }
        }
        None
    }

    /// Reconciles retained focus and hit-test state for matching links.
    pub(crate) fn reconcile_links(&mut self, previous: &Self) {
        for (next, previous) in self.links.iter_mut().zip(&previous.links) {
            if next.target == previous.target {
                next.metadata.reconcile_runtime_state(&previous.metadata);
            }
        }
    }
}

impl Deref for RichText {
    type Target = Text<'static>;

    fn deref(&self) -> &Self::Target {
        &self.text
    }
}

impl From<Text<'static>> for RichText {
    /// Converts Ratatui text into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `text` — Owned Ratatui text to retain.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the visible text.
    fn from(text: Text<'static>) -> Self {
        Self {
            text,
            links: Vec::new(),
        }
    }
}

impl From<Line<'static>> for RichText {
    /// Converts one Ratatui line into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `line` — Owned Ratatui line to retain.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the line.
    fn from(line: Line<'static>) -> Self {
        Self::from(Text::from(line))
    }
}

impl From<Vec<Line<'static>>> for RichText {
    /// Converts owned Ratatui lines into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `lines` — Owned Ratatui lines to retain.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the lines.
    fn from(lines: Vec<Line<'static>>) -> Self {
        Self::from(Text::from(lines))
    }
}

impl<T> From<&[T]> for RichText
where
    T: Into<Line<'static>> + Clone,
{
    /// Copies line-compatible values into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `lines` — Borrowed line-compatible values to copy.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the copied lines.
    fn from(lines: &[T]) -> Self {
        Self::from(Text::from(lines))
    }
}

impl From<Span<'static>> for RichText {
    /// Converts one Ratatui span into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `span` — Owned Ratatui span to retain.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the span.
    fn from(span: Span<'static>) -> Self {
        Self::from(Text::from(Line::from(span)))
    }
}

impl From<Cow<'static, str>> for RichText {
    /// Converts static or owned copy-on-write text into unlinked rich text.
    ///
    /// # Arguments
    ///
    /// * `value` — Static or owned text to retain.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the text.
    fn from(value: Cow<'static, str>) -> Self {
        Self::from(Text::from(value))
    }
}

impl From<String> for RichText {
    /// Converts owned plain text into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `value` — Owned plain text to retain.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the plain text.
    fn from(value: String) -> Self {
        Self::from(Text::raw(value))
    }
}

impl From<&str> for RichText {
    /// Copies borrowed plain text into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `value` — Borrowed plain text to copy.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the copied text.
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

impl From<&String> for RichText {
    /// Copies a borrowed string into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `value` — Borrowed string to copy.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the copied string.
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}

impl PartialEq<Text<'static>> for RichText {
    /// Compares linked rich text with its visible Ratatui text.
    ///
    /// # Arguments
    ///
    /// * `other` — Ratatui text to compare with the visible content.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether the visible text values are equal.
    fn eq(&self, other: &Text<'static>) -> bool {
        &self.text == other
    }
}

impl fmt::Display for RichText {
    /// Formats only the visible text, excluding link destinations and metadata.
    ///
    /// # Arguments
    ///
    /// * `formatter` — Destination formatter for the visible text.
    ///
    /// # Returns
    ///
    /// An empty [`fmt::Result`] after successful formatting.
    ///
    /// # Errors
    ///
    /// Returns [`fmt::Error`] if the destination formatter fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.text, formatter)
    }
}

/// Position of one linked span in retained Ratatui text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LinkedSpan {
    /// Zero-based logical line index.
    pub(crate) line: usize,
    /// Zero-based span index within the logical line.
    pub(crate) span: usize,
}

/// Focus and target metadata for one link embedded in rich text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InlineLink {
    /// Destination opened on activation.
    target: LinkTarget,
    /// Selector and focus metadata for the inline link.
    metadata: StyleMetadata,
    /// Text spans belonging to the link label.
    spans: Vec<LinkedSpan>,
}

impl InlineLink {
    /// Creates one inline link from parsed target and span locations.
    ///
    /// # Arguments
    ///
    /// * `target` — Destination opened on activation.
    /// * `spans` — Text span positions covered by the link label.
    ///
    /// # Returns
    ///
    /// An [`InlineLink`] with fresh `Link` selector metadata.
    pub(crate) fn new(target: LinkTarget, spans: Vec<LinkedSpan>) -> Self {
        Self {
            target,
            metadata: StyleMetadata::new(ViewType::Link),
            spans,
        }
    }
}

/// Resolves styles for links embedded in semantic rich text.
///
/// The semantic container is exposed as the immediate selector ancestor so
/// descendant selectors such as `Paragraph Link` retain their expected shape.
pub(crate) fn resolved_rich_text(
    content: &RichText,
    metadata: &StyleMetadata,
    style: TuiStyle,
    ctx: &mut RenderCtx<'_, '_>,
) -> Text<'static> {
    let mut text = content.text.clone();
    let area = ctx.area();
    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            for link in &content.links {
                let link_style = resolve_style(&link.metadata, ctx).to_ratatui_style();
                for position in &link.spans {
                    if let Some(span) = text
                        .lines
                        .get_mut(position.line)
                        .and_then(|line| line.spans.get_mut(position.span))
                    {
                        span.style = span.style.patch(link_style);
                    }
                }
            }
        },
    );
    text
}

/// Wrapping behavior used by one rich-text renderer.
#[derive(Clone, Copy)]
pub(crate) enum RichTextWrapMode {
    /// Ratatui paragraph word wrapping with leading whitespace preserved.
    Word,
    /// Table-cell wrapping at individual grapheme boundaries.
    Grapheme,
}

/// One visual grapheme retained while computing inline-link geometry.
#[derive(Clone, Copy)]
struct LinkedVisualGrapheme {
    /// Embedded link index owning the grapheme.
    link: Option<usize>,
    /// Terminal-cell width of the grapheme.
    width: u16,
    /// Whether Ratatui treats the grapheme as wrappable whitespace.
    whitespace: bool,
}

/// One wrapped visual row containing link-aware graphemes.
struct LinkedVisualRow {
    /// Graphemes rendered on this row.
    graphemes: Vec<LinkedVisualGrapheme>,
    /// Total terminal-cell width of the row.
    width: u16,
}

/// One visual terminal segment occupied by an inline link.
#[derive(Clone, Copy)]
struct LinkedVisualSegment {
    link: usize,
    row: u16,
    start: u16,
    end: u16,
    line_width: u16,
}

/// Returns link-bearing visual segments using the renderer's wrapping behavior.
///
/// # Arguments
///
/// * `text` — Rich text rendered by the semantic view.
/// * `links` — Inline link metadata associated with text spans.
/// * `width` — Width used to wrap the text.
/// * `wrap_mode` — Wrapping behavior used by the renderer.
///
/// # Returns
///
/// A [`Vec`] containing each visible linked segment in render order.
fn linked_visual_segments(
    text: &Text<'static>,
    links: &[InlineLink],
    width: u16,
    wrap_mode: RichTextWrapMode,
) -> Vec<LinkedVisualSegment> {
    let mut segments = Vec::new();
    for (row, visual_row) in linked_visual_rows(text, links, width, wrap_mode)
        .into_iter()
        .enumerate()
    {
        let row = u16::try_from(row).unwrap_or(u16::MAX);
        let mut pending: Option<LinkedVisualSegment> = None;
        let mut column = 0u16;
        for grapheme in visual_row.graphemes {
            if grapheme.width == 0 {
                continue;
            }

            if let Some(link) = grapheme.link {
                if let Some(segment) = pending.as_mut()
                    && segment.link == link
                    && segment.end == column
                {
                    segment.end = column.saturating_add(grapheme.width);
                } else {
                    finish_linked_segment(&mut segments, &mut pending);
                    pending = Some(LinkedVisualSegment {
                        link,
                        row,
                        start: column,
                        end: column.saturating_add(grapheme.width),
                        line_width: visual_row.width,
                    });
                }
            } else {
                finish_linked_segment(&mut segments, &mut pending);
            }

            column = column.saturating_add(grapheme.width);
        }
        finish_linked_segment(&mut segments, &mut pending);
    }

    segments
}

/// Returns wrapped visual rows while retaining each grapheme's link index.
///
/// # Arguments
///
/// * `text` — Rich text rendered by the semantic view.
/// * `links` — Inline link metadata associated with text spans.
/// * `width` — Width used to wrap the text.
/// * `wrap_mode` — Wrapping behavior used by the renderer.
///
/// # Returns
///
/// A [`Vec`] containing link-aware visual rows in render order.
fn linked_visual_rows(
    text: &Text<'static>,
    links: &[InlineLink],
    width: u16,
    wrap_mode: RichTextWrapMode,
) -> Vec<LinkedVisualRow> {
    let link_spans = links
        .iter()
        .enumerate()
        .flat_map(|(link, inline_link)| {
            inline_link
                .spans
                .iter()
                .copied()
                .map(move |span| (span, link))
        })
        .collect::<Vec<_>>();
    let mut rows = Vec::new();

    for (line_index, line) in text.lines.iter().enumerate() {
        let graphemes = line
            .spans
            .iter()
            .enumerate()
            .flat_map(|(span_index, span)| {
                let link = link_spans
                    .iter()
                    .find(|(position, _)| {
                        position.line == line_index && position.span == span_index
                    })
                    .map(|(_, link)| *link);
                span.styled_graphemes(Style::new())
                    .map(move |grapheme| LinkedVisualGrapheme {
                        link,
                        width: u16::try_from(UnicodeWidthStr::width(grapheme.symbol))
                            .unwrap_or(u16::MAX),
                        whitespace: grapheme.is_whitespace(),
                    })
            })
            .collect::<Vec<_>>();

        rows.extend(match wrap_mode {
            RichTextWrapMode::Word => word_wrapped_visual_rows(graphemes, width),
            RichTextWrapMode::Grapheme => grapheme_wrapped_visual_rows(graphemes, width),
        });
    }

    rows
}

/// Wraps link-aware graphemes like Ratatui's `WordWrapper` with `trim: false`.
fn word_wrapped_visual_rows(
    graphemes: Vec<LinkedVisualGrapheme>,
    width: u16,
) -> Vec<LinkedVisualRow> {
    if width == 0 {
        return vec![LinkedVisualRow {
            graphemes: Vec::new(),
            width: 0,
        }];
    }

    let mut rows = Vec::new();
    let mut pending_line = Vec::new();
    let mut pending_word = Vec::new();
    let mut pending_whitespace: VecDeque<LinkedVisualGrapheme> = VecDeque::new();
    let mut line_width = 0u16;
    let mut word_width = 0u16;
    let mut whitespace_width = 0u16;
    let mut non_whitespace_previous = false;

    for grapheme in graphemes {
        if grapheme.width > width {
            continue;
        }

        let word_found = non_whitespace_previous && grapheme.whitespace;
        let untrimmed_overflow = pending_line.is_empty()
            && word_width
                .saturating_add(whitespace_width)
                .saturating_add(grapheme.width)
                > width;

        if word_found || untrimmed_overflow {
            pending_line.extend(pending_whitespace.drain(..));
            line_width = line_width.saturating_add(whitespace_width);
            pending_line.append(&mut pending_word);
            line_width = line_width.saturating_add(word_width);
            whitespace_width = 0;
            word_width = 0;
        }

        let line_full = line_width >= width;
        let pending_word_overflow = grapheme.width > 0
            && line_width
                .saturating_add(whitespace_width)
                .saturating_add(word_width)
                >= width;

        if line_full || pending_word_overflow {
            let mut remaining_width = width.saturating_sub(line_width);
            rows.push(LinkedVisualRow {
                graphemes: std::mem::take(&mut pending_line),
                width: line_width,
            });
            line_width = 0;

            while let Some(whitespace) = pending_whitespace.front() {
                if whitespace.width > remaining_width {
                    break;
                }
                whitespace_width = whitespace_width.saturating_sub(whitespace.width);
                remaining_width = remaining_width.saturating_sub(whitespace.width);
                pending_whitespace.pop_front();
            }

            if grapheme.whitespace && pending_whitespace.is_empty() {
                continue;
            }
        }

        if grapheme.whitespace {
            whitespace_width = whitespace_width.saturating_add(grapheme.width);
            pending_whitespace.push_back(grapheme);
        } else {
            word_width = word_width.saturating_add(grapheme.width);
            pending_word.push(grapheme);
        }

        non_whitespace_previous = !grapheme.whitespace;
    }

    pending_line.extend(pending_whitespace);
    line_width = line_width.saturating_add(whitespace_width);
    pending_line.append(&mut pending_word);
    line_width = line_width.saturating_add(word_width);
    if !pending_line.is_empty() {
        rows.push(LinkedVisualRow {
            graphemes: pending_line,
            width: line_width,
        });
    }
    if rows.is_empty() {
        rows.push(LinkedVisualRow {
            graphemes: Vec::new(),
            width: 0,
        });
    }

    rows
}

/// Wraps link-aware graphemes at individual grapheme boundaries.
fn grapheme_wrapped_visual_rows(
    graphemes: Vec<LinkedVisualGrapheme>,
    width: u16,
) -> Vec<LinkedVisualRow> {
    if width == 0 {
        return vec![LinkedVisualRow {
            graphemes: Vec::new(),
            width: 0,
        }];
    }

    let mut rows = Vec::new();
    let mut current = Vec::new();
    let mut current_width = 0u16;
    for grapheme in graphemes {
        if grapheme.width > width {
            if !current.is_empty() {
                rows.push(LinkedVisualRow {
                    graphemes: std::mem::take(&mut current),
                    width: current_width,
                });
                current_width = 0;
            }
            continue;
        }
        if current_width.saturating_add(grapheme.width) > width && !current.is_empty() {
            rows.push(LinkedVisualRow {
                graphemes: std::mem::take(&mut current),
                width: current_width,
            });
            current_width = 0;
        }
        current_width = current_width.saturating_add(grapheme.width);
        current.push(grapheme);
    }
    rows.push(LinkedVisualRow {
        graphemes: current,
        width: current_width,
    });
    rows
}

/// Completes the pending inline-link segment.
fn finish_linked_segment(
    segments: &mut Vec<LinkedVisualSegment>,
    pending: &mut Option<LinkedVisualSegment>,
) {
    if let Some(segment) = pending.take() {
        segments.push(segment);
    }
}

/// Returns left padding for a line inside an aligned rich-text area.
fn aligned_line_offset(line_width: u16, width: u16, alignment: CellAlignment) -> u16 {
    match alignment {
        CellAlignment::Left => 0,
        CellAlignment::Center => width.saturating_sub(line_width) / 2,
        CellAlignment::Right => width.saturating_sub(line_width),
    }
}

/// Returns whether destination text begins with an RFC-style URI scheme.
///
/// # Arguments
///
/// * `value` — Destination text to inspect.
///
/// # Returns
///
/// A [`bool`] indicating whether a valid scheme precedes the first colon.
fn has_uri_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

/// Returns whether destination text begins with a Windows drive prefix.
///
/// # Arguments
///
/// * `value` — Destination text to inspect.
///
/// # Returns
///
/// A [`bool`] indicating whether the text begins with a drive letter, colon,
/// and path separator.
fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

/// Opens one actionable target with the operating system's default handler.
///
/// # Arguments
///
/// * `target` — URL or filesystem path to open.
///
/// # Returns
///
/// An [`AppControl::Continue`] value after the handler starts successfully.
///
/// # Errors
///
/// Returns [`Error::LinkOpen`] if a local target is missing or the system
/// handler cannot be started.
pub(crate) fn open_link_target(target: &LinkTarget) -> Result<AppControl> {
    open_link_target_with(target, |argument| open::that(argument))
}

/// Opens one link through an injected launcher.
///
/// # Arguments
///
/// * `target` — URL or filesystem path to validate and open.
/// * `opener` — Launcher receiving the target as an operating-system string.
///
/// # Returns
///
/// An [`AppControl::Continue`] value after the launcher succeeds.
///
/// # Errors
///
/// Returns [`Error::LinkOpen`] if the path is missing, the target is inactive,
/// or `opener` returns an I/O error.
fn open_link_target_with(
    target: &LinkTarget,
    opener: impl FnOnce(&OsStr) -> io::Result<()>,
) -> Result<AppControl> {
    let display = target.display();
    if let LinkTarget::Path(path) = target
        && !path.exists()
    {
        return Err(Error::LinkOpen {
            target: display,
            source: io::Error::new(io::ErrorKind::NotFound, "link target does not exist"),
        });
    }
    let argument = target.as_os_str().ok_or_else(|| Error::LinkOpen {
        target: display.clone(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "link target is not actionable"),
    })?;
    opener(argument).map_err(|source| Error::LinkOpen {
        target: display,
        source,
    })?;
    Ok(AppControl::Continue)
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, io, path::PathBuf};

    use super::{LinkTarget, open_link_target_with};

    /// Verifies string targets distinguish fragments, paths, and absolute URIs.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// #section
    /// guide.md
    /// https://example.com
    /// mailto:team@example.com
    /// ```
    ///
    /// # Assertions
    ///
    /// - Empty and hash-prefixed targets become inactive fragments.
    /// - Relative and Windows drive-prefixed text become filesystem paths.
    /// - HTTP and mail targets become URLs.
    #[test]
    fn string_targets_are_classified() {
        assert_eq!(LinkTarget::from(""), LinkTarget::Fragment(String::new()));
        assert_eq!(
            LinkTarget::from("#section"),
            LinkTarget::Fragment("#section".to_owned())
        );
        assert_eq!(
            LinkTarget::from("guide.md"),
            LinkTarget::Path(PathBuf::from("guide.md"))
        );
        assert_eq!(
            LinkTarget::from("C:/guide.md"),
            LinkTarget::Path(PathBuf::from("C:/guide.md"))
        );
        assert_eq!(
            LinkTarget::from(r"C:\guide.md"),
            LinkTarget::Path(PathBuf::from(r"C:\guide.md"))
        );
        assert_eq!(
            LinkTarget::from("https://example.com"),
            LinkTarget::Url("https://example.com".to_owned())
        );
        assert_eq!(
            LinkTarget::from("mailto:team@example.com"),
            LinkTarget::Url("mailto:team@example.com".to_owned())
        );
    }

    /// Verifies launcher success and failure remain deterministic in tests.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// https://example.com
    /// ```
    ///
    /// # Assertions
    ///
    /// - A successful injected launcher receives the URL and continues.
    /// - An injected I/O failure becomes a target-aware link-open error.
    #[test]
    fn link_opening_uses_injected_launcher() {
        let called = Cell::new(false);
        let target = LinkTarget::from("https://example.com");
        let result = open_link_target_with(&target, |argument| {
            assert_eq!(argument, "https://example.com");
            called.set(true);
            Ok(())
        });
        assert_eq!(result.unwrap(), crate::AppControl::Continue);
        assert!(called.get());

        let error = open_link_target_with(&target, |_| Err(io::Error::other("launcher failed")))
            .unwrap_err();
        assert!(error.to_string().contains("https://example.com"));
    }
}
