//! Path-backed terminal image view.

use std::path::PathBuf;

use ratatui::layout::Rect;

use crate::view::core::{
    capabilities::impl_styled_view,
    measurement::{AvailableSpace, measure_fixed},
    render::resolve_style,
};
use crate::view::{StyleMetadata, View, ViewType};
use crate::{LayoutSize, TuiSize, app::Result, component::RenderCtx};

/// Source data used by an image view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImageSource {
    /// Image loaded from a filesystem path.
    Path(PathBuf),
}

impl From<PathBuf> for ImageSource {
    /// Converts a path buffer into a path-backed image source.
    ///
    /// # Arguments
    ///
    /// * `value` — Path to load when rendering the image.
    ///
    /// # Returns
    ///
    /// An [`ImageSource::Path`] containing `value`.
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<&str> for ImageSource {
    /// Converts a borrowed path into a path-backed image source.
    ///
    /// # Arguments
    ///
    /// * `value` — Path to copy into the image source.
    ///
    /// # Returns
    ///
    /// An [`ImageSource::Path`] containing `value`.
    fn from(value: &str) -> Self {
        Self::Path(PathBuf::from(value))
    }
}

impl From<String> for ImageSource {
    /// Converts an owned path string into a path-backed image source.
    ///
    /// # Arguments
    ///
    /// * `value` — Path string to move into the image source.
    ///
    /// # Returns
    ///
    /// An [`ImageSource::Path`] containing `value`.
    fn from(value: String) -> Self {
        Self::Path(PathBuf::from(value))
    }
}

/// Path-backed terminal image with deterministic text fallback.
#[derive(Debug, PartialEq)]
pub struct ImageView {
    /// Image source to render.
    pub(crate) source: ImageSource,
    /// Optional fallback text.
    pub(crate) alt: Option<String>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

impl ImageView {
    /// Stores fallback text for unavailable image rendering.
    ///
    /// # Arguments
    ///
    /// * `alt` — Text displayed when terminal image rendering is unavailable.
    ///
    /// # Returns
    ///
    /// This image view with fallback text configured.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }
}

/// Creates an image view.
///
/// # Arguments
///
/// * `source` — Filesystem-backed image source.
///
/// # Returns
///
/// An [`ImageView`] with no fallback text.
pub fn image(source: impl Into<ImageSource>) -> ImageView {
    ImageView {
        source: source.into(),
        alt: None,
        metadata: StyleMetadata::new(ViewType::Image),
    }
}

impl ImageView {
    /// Returns the configured image source.
    pub fn source(&self) -> &ImageSource {
        &self.source
    }

    /// Returns optional fallback text.
    pub fn alt_text(&self) -> Option<&str> {
        self.alt.as_deref()
    }
}

pub(crate) fn image_render_area(area: Rect, image_size: Option<TuiSize>) -> Rect {
    let Some(size) = image_size else {
        return area;
    };

    Rect {
        width: size.width.min(area.width),
        height: size.height.min(area.height),
        ..area
    }
}

impl View for ImageView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let style = resolve_style(&self.metadata, ctx);
        let ImageSource::Path(path) = &self.source;
        let area = if let Some(geometry) = ctx.active_layout_geometry(&self.metadata) {
            ctx.with_area(geometry.border_box, |ctx| {
                ctx.render_widget(style.to_block());
            });
            image_render_area(geometry.content_box, style.image_size)
        } else {
            image_render_area(ctx.area(), style.image_size)
        };
        ctx.with_area(area, |ctx| {
            ctx.render_terminal_image_path(path, self.alt.as_deref(), style.to_ratatui_style());
        });
        Ok(())
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        _available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let size = resolve_style(&self.metadata, ctx)
            .image_size
            .unwrap_or_else(|| TuiSize::new(1, 1));
        measure_fixed(
            LayoutSize::new(f32::from(size.width), f32::from(size.height)),
            known_dimensions,
        )
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

impl_styled_view!(ImageView);
