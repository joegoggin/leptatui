//! Gauge-style progress-bar view.

use ratatui::{text::Span, widgets::Gauge};

use crate::view::core::{
    capabilities::impl_styled_view,
    measurement::{AvailableSpace, measure_fixed},
    render::resolve_style,
};
use crate::view::{StyleMetadata, View, ViewType};
use crate::{LayoutSize, app::Result, component::RenderCtx};
use unicode_width::UnicodeWidthStr;

/// Returns a clamped progress value safe for Ratatui gauge rendering.
pub(crate) fn clamped_progress_value(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Gauge-style progress indicator.
#[derive(Debug, PartialEq)]
pub struct ProgressBarView {
    /// Clamped completion ratio.
    pub(crate) value: f64,
    /// Optional gauge label.
    pub(crate) label: Option<String>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

impl ProgressBarView {
    /// Stores label text rendered over the gauge.
    ///
    /// # Arguments
    ///
    /// * `label` — Text rendered over the gauge.
    ///
    /// # Returns
    ///
    /// This progress bar with a label configured.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Creates a progress bar.
///
/// # Arguments
///
/// * `value` — Completion ratio, clamped to `0.0..=1.0`.
///
/// # Returns
///
/// A [`ProgressBarView`] with no label.
pub fn progress_bar(value: f64) -> ProgressBarView {
    ProgressBarView {
        value: clamped_progress_value(value),
        label: None,
        metadata: StyleMetadata::new(ViewType::ProgressBar),
    }
}

impl ProgressBarView {
    /// Returns the clamped completion ratio.
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// Returns the optional gauge label.
    pub fn label_text(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

impl View for ProgressBarView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let style = resolve_style(&self.metadata, ctx);
        let ratatui_style = style.to_ratatui_style();
        let mut gauge = Gauge::default()
            .ratio(clamped_progress_value(self.value))
            .style(ratatui_style)
            .gauge_style(ratatui_style);
        if let Some(label) = self.label.as_deref() {
            gauge = gauge.label(Span::styled(label, ratatui_style));
        }
        ctx.render_widget(gauge);
        Ok(())
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        _available_space: LayoutSize<AvailableSpace>,
        _ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let width = self.label.as_deref().map_or(1, |label| {
            u16::try_from(UnicodeWidthStr::width(label))
                .unwrap_or(u16::MAX)
                .max(1)
        });
        measure_fixed(LayoutSize::new(f32::from(width), 1.0), known_dimensions)
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

impl_styled_view!(ProgressBarView);
