//! Intrinsic terminal-cell measurement primitives.
//!
//! Measurement is independent of painting and uses engine-neutral values that
//! can be adapted to a computed layout tree without exposing its implementation.

use ratatui::text::Text;
use unicode_width::UnicodeWidthStr;

use crate::{LayoutSize, RenderCtx, TuiStyle, View};

use super::render::{line_count_height, semantic_paragraph};

/// Space available to a view along one layout axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AvailableSpace {
    /// A finite soft constraint measured in terminal cells.
    Definite(f32),
    /// The smallest intrinsic size that avoids avoidable overflow.
    MinContent,
    /// The preferred intrinsic size without soft wrapping.
    MaxContent,
}

impl AvailableSpace {
    /// Returns a definite constraint after discarding invalid or negative cells.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing a finite non-negative cell count.
    pub(crate) fn definite(self) -> Option<f32> {
        match self {
            Self::Definite(value) => Some(sanitize_cells(value)),
            Self::MinContent | Self::MaxContent => None,
        }
    }
}

/// Returns a finite non-negative terminal-cell measurement.
///
/// # Arguments
///
/// * `value` — Floating-point terminal-cell value to sanitize.
///
/// # Returns
///
/// A finite `f32` clamped to the Ratatui coordinate range.
pub(crate) fn sanitize_cells(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, f32::from(u16::MAX))
    } else {
        0.0
    }
}

/// Converts a floating-point terminal-cell value into a Ratatui dimension.
///
/// Fractional cells are rounded down because a partial terminal cell cannot be
/// used for wrapping.
///
/// # Arguments
///
/// * `value` — Floating-point terminal-cell value to convert.
///
/// # Returns
///
/// A saturated `u16` terminal-cell count.
pub(crate) fn cells_to_u16(value: f32) -> u16 {
    sanitize_cells(value).floor() as u16
}

/// Resolves an intrinsic axis from known and available-space constraints.
///
/// # Arguments
///
/// * `known` — Exact dimension supplied by the parent layout.
/// * `available` — Soft available-space constraint.
/// * `min_content` — Smallest intrinsic size for this axis.
/// * `max_content` — Preferred intrinsic size for this axis.
///
/// # Returns
///
/// A finite terminal-cell size for the axis.
pub(crate) fn resolve_intrinsic_axis(
    known: Option<f32>,
    available: AvailableSpace,
    min_content: f32,
    max_content: f32,
) -> f32 {
    if let Some(known) = known {
        return sanitize_cells(known);
    }

    let min_content = sanitize_cells(min_content);
    let max_content = sanitize_cells(max_content).max(min_content);
    match available {
        AvailableSpace::Definite(value) => sanitize_cells(value).min(max_content).max(min_content),
        AvailableSpace::MinContent => min_content,
        AvailableSpace::MaxContent => max_content,
    }
}

/// Measures a fixed-size leaf while honoring exact known dimensions.
///
/// # Arguments
///
/// * `intrinsic` — Preferred fixed terminal-cell size.
/// * `known_dimensions` — Exact dimensions supplied by parent layout.
///
/// # Returns
///
/// A [`LayoutSize`] containing the resolved size.
pub(crate) fn measure_fixed(
    intrinsic: LayoutSize<f32>,
    known_dimensions: LayoutSize<Option<f32>>,
) -> LayoutSize<f32> {
    LayoutSize::new(
        known_dimensions
            .width
            .map_or_else(|| sanitize_cells(intrinsic.width), sanitize_cells),
        known_dimensions
            .height
            .map_or_else(|| sanitize_cells(intrinsic.height), sanitize_cells),
    )
}

/// Measures a view's intrinsic height at the current rendering width.
///
/// # Arguments
///
/// * `view` — View whose two-axis measurement supplies the height.
/// * `ctx` — Rendering context containing the current available area.
///
/// # Returns
///
/// A saturated `u16` height measured through [`View::measure`].
pub(crate) fn measure_view_height(view: &dyn View, ctx: &mut RenderCtx<'_, '_>) -> u16 {
    if view
        .style_metadata()
        .is_some_and(crate::StyleMetadata::is_layout_hidden)
    {
        return 0;
    }

    let area = ctx.area();
    let measured = view.measure(
        LayoutSize::new(Some(f32::from(area.width)), None),
        LayoutSize::new(
            AvailableSpace::Definite(f32::from(area.width)),
            AvailableSpace::Definite(f32::from(area.height)),
        ),
        ctx,
    );
    cells_to_u16(measured.height)
}

/// Measures word-wrapped rich text without painting it.
///
/// # Arguments
///
/// * `content` — Rich text whose terminal-cell geometry is measured.
/// * `style` — Resolved text style used by the render-equivalent paragraph.
/// * `known_dimensions` — Exact dimensions supplied by parent layout.
/// * `available_space` — Soft constraints used for intrinsic width selection.
///
/// # Returns
///
/// A [`LayoutSize`] containing intrinsic width and wrapped height.
pub(crate) fn measure_rich_text(
    content: &Text<'static>,
    style: TuiStyle,
    known_dimensions: LayoutSize<Option<f32>>,
    available_space: LayoutSize<AvailableSpace>,
) -> LayoutSize<f32> {
    if let (Some(width), Some(height)) = (known_dimensions.width, known_dimensions.height) {
        return LayoutSize::new(sanitize_cells(width), sanitize_cells(height));
    }

    let (min_width, max_width) = rich_text_intrinsic_widths(content);
    let width = resolve_intrinsic_axis(
        known_dimensions.width,
        available_space.width,
        f32::from(min_width),
        f32::from(max_width),
    );
    let wrapping_width = known_dimensions
        .width
        .or_else(|| available_space.width.definite())
        .map_or(width, sanitize_cells);
    let cell_width = cells_to_u16(wrapping_width);
    let natural_height = if content.lines.is_empty() || cell_width == 0 {
        0
    } else {
        line_count_height(semantic_paragraph(content, style).line_count(cell_width)).max(1)
    };
    let height = known_dimensions
        .height
        .map_or(f32::from(natural_height), sanitize_cells);

    LayoutSize::new(width, height)
}

/// Returns min-content and max-content widths for rich terminal text.
///
/// # Arguments
///
/// * `content` — Rich text to inspect.
///
/// # Returns
///
/// A tuple containing min-content and max-content widths.
pub(crate) fn rich_text_intrinsic_widths(content: &Text<'static>) -> (u16, u16) {
    let mut min_width = 0usize;
    let mut max_width = 0usize;

    for line in &content.lines {
        let mut line_width = 0usize;
        let mut word = String::new();
        for span in &line.spans {
            line_width = line_width.saturating_add(UnicodeWidthStr::width(span.content.as_ref()));
            for character in span.content.chars() {
                if character.is_whitespace() {
                    min_width = min_width.max(UnicodeWidthStr::width(word.as_str()));
                    word.clear();
                } else {
                    word.push(character);
                }
            }
        }
        min_width = min_width.max(UnicodeWidthStr::width(word.as_str()));
        max_width = max_width.max(line_width);
    }

    (
        u16::try_from(min_width).unwrap_or(u16::MAX),
        u16::try_from(max_width).unwrap_or(u16::MAX),
    )
}

/// Resolves the default intrinsic size for a render-only custom view.
///
/// # Arguments
///
/// * `known_dimensions` — Exact dimensions supplied by parent layout.
/// * `available_space` — Soft constraints used when an axis is unknown.
///
/// # Returns
///
/// A one-cell intrinsic [`LayoutSize`] with known axes applied.
pub(crate) fn measure_default(
    known_dimensions: LayoutSize<Option<f32>>,
    available_space: LayoutSize<AvailableSpace>,
) -> LayoutSize<f32> {
    LayoutSize::new(
        resolve_intrinsic_axis(known_dimensions.width, available_space.width, 1.0, 1.0),
        resolve_intrinsic_axis(known_dimensions.height, available_space.height, 1.0, 1.0),
    )
}
