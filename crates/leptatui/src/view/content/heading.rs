//! Markdown-style semantic heading view.

use ratatui::{layout::Rect, widgets::Paragraph};

use crate::view::core::{
    capabilities::{impl_styled_view, impl_textual_view},
    render::{line_count_height, resolve_style, semantic_paragraph},
};
use crate::view::{CellAlignment, StyleMetadata, View, ViewType, link::resolved_rich_text};
use crate::{
    RichText, TuiStyle,
    app::{AppControl, Result},
    component::{FocusedControl, RenderCtx},
};

/// One-based semantic heading level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeadingLevel {
    /// First-level heading.
    H1,
    /// Second-level heading.
    H2,
    /// Third-level heading.
    H3,
    /// Fourth-level heading.
    H4,
    /// Fifth-level heading.
    H5,
    /// Sixth-level heading.
    H6,
}

impl HeadingLevel {
    /// Returns the numeric heading level.
    ///
    /// # Returns
    ///
    /// A [`u16`] from one through six.
    pub const fn number(self) -> u16 {
        match self {
            Self::H1 => 1,
            Self::H2 => 2,
            Self::H3 => 3,
            Self::H4 => 4,
            Self::H5 => 5,
            Self::H6 => 6,
        }
    }

    /// Returns the semantic selector identity for this heading level.
    ///
    /// # Returns
    ///
    /// A built-in [`ViewType`] heading identity.
    pub const fn view_type(self) -> ViewType {
        match self {
            Self::H1 => ViewType::H1,
            Self::H2 => ViewType::H2,
            Self::H3 => ViewType::H3,
            Self::H4 => ViewType::H4,
            Self::H5 => ViewType::H5,
            Self::H6 => ViewType::H6,
        }
    }
}

/// Markdown-style semantic heading.
#[derive(Debug, PartialEq)]
pub struct HeadingView {
    /// Rich heading content.
    pub(crate) content: RichText,
    /// Heading level controlling markers and selector identity.
    pub(crate) level: HeadingLevel,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a semantic heading with the requested level.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
/// * `level` — Semantic heading level and selector identity.
///
/// # Returns
///
/// A [`HeadingView`] containing `content` at `level`.
fn heading(content: impl Into<RichText>, level: HeadingLevel) -> HeadingView {
    HeadingView {
        content: content.into(),
        level,
        metadata: StyleMetadata::new(level.view_type()),
    }
}

/// Creates a first-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A first-level [`HeadingView`].
pub fn h1(content: impl Into<RichText>) -> HeadingView {
    heading(content, HeadingLevel::H1)
}

/// Creates a second-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A second-level [`HeadingView`].
pub fn h2(content: impl Into<RichText>) -> HeadingView {
    heading(content, HeadingLevel::H2)
}

/// Creates a third-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A third-level [`HeadingView`].
pub fn h3(content: impl Into<RichText>) -> HeadingView {
    heading(content, HeadingLevel::H3)
}

/// Creates a fourth-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A fourth-level [`HeadingView`].
pub fn h4(content: impl Into<RichText>) -> HeadingView {
    heading(content, HeadingLevel::H4)
}

/// Creates a fifth-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A fifth-level [`HeadingView`].
pub fn h5(content: impl Into<RichText>) -> HeadingView {
    heading(content, HeadingLevel::H5)
}

/// Creates a sixth-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A sixth-level [`HeadingView`].
pub fn h6(content: impl Into<RichText>) -> HeadingView {
    heading(content, HeadingLevel::H6)
}

impl HeadingView {
    /// Returns this heading's semantic level.
    pub const fn level(&self) -> HeadingLevel {
        self.level
    }
}

/// Returns the horizontal offset after a Markdown heading marker.
fn heading_content_offset(level: u16) -> u16 {
    level.saturating_add(1)
}

/// Renders a Markdown-style semantic heading with a hanging content indent.
///
/// The marker occupies only the first row while wrapped content remains aligned
/// beneath the heading text.
///
/// # Arguments
///
/// * `content` — Rich heading text to render.
/// * `metadata` — Selector metadata used to resolve the heading style.
/// * `level` — One-based semantic heading level.
/// * `ctx` — Rendering context containing the target area and stylesheets.
fn render_heading(
    content: &RichText,
    metadata: &StyleMetadata,
    level: u16,
    ctx: &mut RenderCtx<'_, '_>,
) {
    let style = resolve_style(metadata, ctx);
    let rendered = resolved_rich_text(content, metadata, style, ctx);
    let area = ctx.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let content_offset = heading_content_offset(level).min(area.width);
    let marker = format!("{} ", "#".repeat(usize::from(level)));
    let marker_area = Rect {
        width: content_offset,
        height: 1,
        ..area
    };
    ctx.with_area(marker_area, |ctx| {
        ctx.render_widget(Paragraph::new(marker).style(style.to_ratatui_style()));
    });

    if content_offset < area.width {
        let content_area = Rect {
            x: area.x.saturating_add(content_offset),
            width: area.width.saturating_sub(content_offset),
            ..area
        };
        ctx.with_area(content_area, |ctx| {
            ctx.render_widget(semantic_paragraph(&rendered, style));
        });
        content.record_link_hit_areas(content_area, content_area.width, CellAlignment::Left, ctx);
    } else if area.height > 1 {
        let content_area = Rect {
            y: area.y.saturating_add(1),
            height: area.height.saturating_sub(1),
            ..area
        };
        ctx.with_area(content_area, |ctx| {
            ctx.render_widget(semantic_paragraph(&rendered, style));
        });
        content.record_link_hit_areas(content_area, content_area.width, CellAlignment::Left, ctx);
    }
    content.clear_link_scroll_requests();
}

/// Returns the minimum height required by a Markdown-style semantic heading.
///
/// # Arguments
///
/// * `content` — Rich heading text to measure.
/// * `style` — Resolved style applied beneath rich-text spans.
/// * `level` — One-based semantic heading level.
/// * `width` — Available heading width in terminal cells.
///
/// # Returns
///
/// A [`u16`] row count that includes wrapping after the heading marker.
fn heading_min_height(content: &RichText, style: TuiStyle, level: u16, width: u16) -> u16 {
    let content_width = width.saturating_sub(heading_content_offset(level));
    if content_width == 0 {
        if width == 0 {
            return 0;
        }

        return 1u16.saturating_add(line_count_height(
            semantic_paragraph(content.text(), style).line_count(width),
        ));
    }

    line_count_height(semantic_paragraph(content.text(), style).line_count(content_width)).max(1)
}

impl View for HeadingView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        render_heading(&self.content, &self.metadata, self.level.number(), ctx);
        self.metadata.clear_scroll_to_anchor_request();
        Ok(())
    }

    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        let style = resolve_style(&self.metadata, ctx);
        heading_min_height(&self.content, style, self.level.number(), ctx.area().width)
    }

    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn reconcile(&mut self, previous: &dyn View) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.content.reconcile_links(&previous.content);
        }
    }
    fn can_reconcile_from(&self, previous: &dyn View) -> bool {
        previous
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|previous| self.level == previous.level)
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
    ) -> Option<usize> {
        self.content.focusable_index_at_position(column, row, index)
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        let area = ctx.area();
        if self.metadata.scroll_to_anchor_requested() {
            return Some((0, u32::from(area.height)));
        }
        if !self.content.focused_link_requested_scroll() {
            return None;
        }

        let offset = heading_content_offset(self.level.number()).min(area.width);
        let content_width = area.width.saturating_sub(offset);
        if content_width > 0 {
            self.content
                .focused_link_span(content_width)
                .map(|span| span.into_tuple())
        } else {
            self.content
                .focused_link_span(area.width)
                .map(|span| span.offset_by(1).into_tuple())
        }
    }

    fn __activate_focused_button(&self) -> Result<Option<AppControl>> {
        self.content.activate_focused_link()
    }

    fn __focused_control(&self) -> Option<FocusedControl> {
        self.content.focused_control()
    }

    fn __focused_link_target(&self) -> Option<crate::LinkTarget> {
        self.content.focused_link_target()
    }

    fn __clear_hit_areas(&self) {
        self.metadata.clear_hit_areas();
        self.content.clear_link_hit_areas();
    }
}

impl_styled_view!(HeadingView);
impl_textual_view!(HeadingView);
