//! Rich-text content with optional focusable link ranges.

use std::{borrow::Cow, fmt, ops::Deref};

use ratatui::{
    layout::Rect,
    text::{Line, Span, Text},
};

use crate::{
    TuiStyle,
    app::{AppControl, Result},
    component::{FocusedControl, RenderCtx},
};

use super::{
    geometry::{RichTextWrapMode, aligned_line_offset, linked_visual_segments},
    target::{LinkTarget, activate_link_target},
    visited::sync_visited,
};
use crate::view::{
    CellAlignment,
    core::{
        metadata::{StyleMetadata, ViewType},
        render::{VerticalSpan, resolve_style},
    },
};

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
    ///
    /// # Returns
    ///
    /// A [`usize`] count of links that participate in focus traversal.
    pub(crate) fn focusable_count(&self) -> usize {
        self.links
            .iter()
            .filter(|link| link.target.is_actionable())
            .count()
    }

    /// Returns the focused embedded-link index during flattened traversal.
    ///
    /// The traversal index advances once for every actionable embedded link.
    ///
    /// # Arguments
    ///
    /// * `index` — Running flattened focus index to inspect and advance.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the focused link's flattened index.
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
    ///
    /// The selected link requests scrolling into view while all other embedded
    /// links clear retained focus and scroll requests.
    ///
    /// # Arguments
    ///
    /// * `target` — Flattened focus index that should become focused.
    /// * `index` — Running flattened focus index to inspect and advance.
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
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the activated link's control value, or [`None`]
    /// when no actionable embedded link is focused.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::LinkOpen`] if the focused target cannot be
    /// opened.
    pub(crate) fn activate_focused_link(&self) -> Result<Option<AppControl>> {
        for link in &self.links {
            if link.metadata.is_focused() && link.target.is_actionable() {
                let control = activate_link_target(&link.metadata, &link.target)?;
                return Ok(Some(control));
            }
        }
        Ok(None)
    }

    /// Returns the target of the focused actionable embedded link.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing a clone of the focused link target.
    pub(crate) fn focused_link_target(&self) -> Option<LinkTarget> {
        self.links
            .iter()
            .find(|link| link.metadata.is_focused() && link.target.is_actionable())
            .map(|link| link.target.clone())
    }

    /// Returns focused-control metadata for an embedded link.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing [`FocusedControl::Link`] when an actionable
    /// embedded link is focused.
    pub(crate) fn focused_control(&self) -> Option<FocusedControl> {
        self.links
            .iter()
            .any(|link| link.metadata.is_focused() && link.target.is_actionable())
            .then_some(FocusedControl::Link)
    }

    /// Returns whether a focused embedded link requested scrolling.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether the focused link should scroll into view.
    pub(crate) fn focused_link_requested_scroll(&self) -> bool {
        self.links
            .iter()
            .any(|link| link.metadata.is_focused() && link.metadata.scroll_into_view_requested())
    }

    /// Returns the wrapped row span of the focused embedded link.
    ///
    /// # Arguments
    ///
    /// * `width` — Terminal-cell width used for word wrapping.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the focused link's half-open vertical span.
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
            if let Some(link) = self.links.get(segment.link) {
                ctx.push_metadata_hit_area(&link.metadata, hit_area);
            }
        }
    }

    /// Returns the embedded-link index under a terminal position.
    ///
    /// The traversal index advances once for every actionable embedded link.
    ///
    /// # Arguments
    ///
    /// * `column` — Zero-based terminal column to hit test.
    /// * `row` — Zero-based terminal row to hit test.
    /// * `index` — Running flattened focus index to inspect and advance.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the flattened index and global paint ordinal
    /// of the frontmost link under the position.
    pub(crate) fn focusable_index_at_position(
        &self,
        column: u16,
        row: u16,
        index: &mut usize,
    ) -> Option<(usize, u64)> {
        let mut frontmost = None;
        for link in &self.links {
            if !link.target.is_actionable() {
                continue;
            }
            let current = *index;
            *index = index.saturating_add(1);
            if link.metadata.contains_hit_position(column, row)
                && let Some(order) = link.metadata.paint_order()
                && frontmost.is_none_or(|(_, current_order)| order > current_order)
            {
                frontmost = Some((current, order));
            }
        }
        frontmost
    }

    /// Reconciles retained focus and hit-test state for matching links.
    ///
    /// Links are paired in source order and retain runtime state only when
    /// their targets match.
    ///
    /// # Arguments
    ///
    /// * `previous` — Previously rendered rich text supplying runtime state.
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
    pub(super) spans: Vec<LinkedSpan>,
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
///
/// # Arguments
///
/// * `content` — Rich text and embedded-link metadata to style.
/// * `metadata` — Selector metadata for the containing semantic view.
/// * `style` — Resolved style inherited from the containing view.
/// * `ctx` — Render context used to resolve descendant link selectors.
///
/// # Returns
///
/// A [`Text`] value with resolved link styles patched onto linked spans.
pub(crate) fn resolved_rich_text(
    content: &RichText,
    metadata: &StyleMetadata,
    style: &TuiStyle,
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
                sync_visited(&link.metadata, &link.target);
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

/// Implements embedded-link interaction for a view with `content` and `metadata` fields.
macro_rules! impl_rich_text_view {
    () => {
        fn reconcile(&mut self, previous: &dyn $crate::view::View) {
            if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
                self.content.reconcile_links(&previous.content);
            }
        }

        fn __focusable_count(&self) -> usize {
            self.content.focusable_count()
        }

        fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
            self.content.focused_index_inner(index)
        }

        fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
            self.content.set_focus_by_index_inner(target, index);
        }

        fn __focusable_index_at_position_inner(
            &self,
            column: u16,
            row: u16,
            index: &mut usize,
        ) -> Option<(usize, u64)> {
            self.content.focusable_index_at_position(column, row, index)
        }

        fn __activate_focused_button(&self) -> $crate::Result<Option<$crate::AppControl>> {
            self.content.activate_focused_link()
        }

        fn __focused_control(&self) -> Option<$crate::component::FocusedControl> {
            self.content.focused_control()
        }

        fn __focused_link_target(&self) -> Option<$crate::LinkTarget> {
            self.content.focused_link_target()
        }

        fn __clear_hit_areas(&self) {
            self.metadata.clear_hit_areas();
            self.content.clear_link_hit_areas();
        }
    };
}
pub(crate) use impl_rich_text_view;
