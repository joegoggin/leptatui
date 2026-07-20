//! Markdown-style semantic heading view.

use ratatui::{layout::Rect, text::Text, widgets::Paragraph};

use crate::view::core::{
    capabilities::{impl_styled_view, impl_textual_view},
    render::{line_count_height, resolve_style, semantic_paragraph},
};
use crate::view::{StyleMetadata, View, ViewType};
use crate::{TuiStyle, app::Result, component::RenderCtx};

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
    pub(crate) content: Text<'static>,
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
fn heading(content: impl Into<Text<'static>>, level: HeadingLevel) -> HeadingView {
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
pub fn h1(content: impl Into<Text<'static>>) -> HeadingView {
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
pub fn h2(content: impl Into<Text<'static>>) -> HeadingView {
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
pub fn h3(content: impl Into<Text<'static>>) -> HeadingView {
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
pub fn h4(content: impl Into<Text<'static>>) -> HeadingView {
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
pub fn h5(content: impl Into<Text<'static>>) -> HeadingView {
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
pub fn h6(content: impl Into<Text<'static>>) -> HeadingView {
    heading(content, HeadingLevel::H6)
}

impl HeadingView {
    /// Returns this heading's semantic level.
    pub const fn level(&self) -> HeadingLevel {
        self.level
    }
}

fn heading_content_offset(level: u16) -> u16 {
    level.saturating_add(1)
}

/// Returns the one-based level of a semantic heading view.
///
/// # Arguments
///
/// * `view` — Semantic heading view to classify.
///
/// # Returns
///
/// A [`u16`] containing the heading level from one through six.
///
/// # Panics
///
/// Panics if `view` is not a semantic heading variant.
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
    content: &Text<'static>,
    metadata: &StyleMetadata,
    level: u16,
    ctx: &mut RenderCtx<'_, '_>,
) {
    let style = resolve_style(metadata, ctx);
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
            ctx.render_widget(semantic_paragraph(content, style));
        });
    } else if area.height > 1 {
        let content_area = Rect {
            y: area.y.saturating_add(1),
            height: area.height.saturating_sub(1),
            ..area
        };
        ctx.with_area(content_area, |ctx| {
            ctx.render_widget(semantic_paragraph(content, style));
        });
    }
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
fn heading_min_height(content: &Text<'static>, style: TuiStyle, level: u16, width: u16) -> u16 {
    let content_width = width.saturating_sub(heading_content_offset(level));
    if content_width == 0 {
        if width == 0 {
            return 0;
        }

        return 1u16.saturating_add(line_count_height(
            semantic_paragraph(content, style).line_count(width),
        ));
    }

    line_count_height(semantic_paragraph(content, style).line_count(content_width)).max(1)
}

impl View for HeadingView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        render_heading(&self.content, &self.metadata, self.level.number(), ctx);
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
}

impl_styled_view!(HeadingView);
impl_textual_view!(HeadingView);
