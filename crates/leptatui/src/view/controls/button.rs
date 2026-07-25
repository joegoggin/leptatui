//! Focusable bordered button view.

use std::{fmt, rc::Rc};

use ratatui::widgets::Paragraph;

use crate::view::core::{
    capabilities::impl_styled_view,
    measurement::{AvailableSpace, measure_fixed},
    render::{
        horizontal_border_columns, horizontal_padding_columns, resolve_style, vertical_border_rows,
        vertical_padding_rows,
    },
};
use crate::view::{StyleMetadata, View, ViewType};
use crate::{
    Borders, LayoutSize,
    app::{AppControl, Result},
    component::{FocusedControl, RenderCtx},
};
use unicode_width::UnicodeWidthStr;

/// Shared callback invoked when a button is activated.
pub type ButtonAction = Rc<dyn Fn() -> AppControl>;

/// Focusable bordered button.
pub struct ButtonView {
    /// Centered button label.
    pub(crate) label: String,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
    /// Optional activation callback.
    pub(crate) on_press: Option<ButtonAction>,
}

impl ButtonView {
    /// Stores an activation callback.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback invoked when this button is activated.
    ///
    /// # Returns
    ///
    /// This button with the callback configured.
    pub fn on_press(mut self, action: impl Fn() -> AppControl + 'static) -> Self {
        self.on_press = Some(Rc::new(action));
        self
    }
}

/// Creates an interactive button.
///
/// # Arguments
///
/// * `label` — Text displayed inside the button.
///
/// # Returns
///
/// A [`ButtonView`] with no activation callback.
pub fn button(label: impl Into<String>) -> ButtonView {
    ButtonView {
        label: label.into(),
        metadata: StyleMetadata::new(ViewType::Button),
        on_press: None,
    }
}

impl PartialEq for ButtonView {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label
            && self.metadata == other.metadata
            && super::actions_equal(&self.on_press, &other.on_press)
    }
}

impl fmt::Debug for ButtonView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ButtonView")
            .field("label", &self.label)
            .field("metadata", &self.metadata)
            .field("has_on_press", &self.on_press.is_some())
            .finish()
    }
}

impl ButtonView {
    /// Returns the button label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns whether an activation callback is configured.
    pub fn has_on_press(&self) -> bool {
        self.on_press.is_some()
    }
}

impl View for ButtonView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let style = resolve_style(&self.metadata, ctx);
        if let Some(geometry) = ctx.active_layout_geometry(&self.metadata) {
            ctx.with_area(geometry.border_box, |ctx| {
                ctx.render_widget(style.to_block_with_default_borders(Borders::ALL));
            });
            ctx.with_area(geometry.content_box, |ctx| {
                ctx.render_widget(
                    Paragraph::new(self.label.as_str())
                        .centered()
                        .style(style.to_ratatui_style()),
                );
            });
        } else {
            ctx.render_widget(
                Paragraph::new(self.label.as_str())
                    .centered()
                    .style(style.to_ratatui_style())
                    .block(style.to_block_with_default_borders(Borders::ALL)),
            );
        }
        ctx.record_metadata_hit_area(&self.metadata);
        self.metadata.clear_scroll_into_view_request();
        Ok(())
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        _available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let style = resolve_style(&self.metadata, ctx);
        let borders = style.borders.unwrap_or(Borders::ALL);
        let width = u16::try_from(UnicodeWidthStr::width(self.label.as_str()))
            .unwrap_or(u16::MAX)
            .saturating_add(horizontal_border_columns(borders))
            .saturating_add(horizontal_padding_columns(style.padding));
        let height = 1u16
            .saturating_add(vertical_border_rows(borders))
            .saturating_add(vertical_padding_rows(style.padding));
        measure_fixed(
            LayoutSize::new(f32::from(width), f32::from(height)),
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
    fn __focusable_count(&self) -> usize {
        1
    }

    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        let current = *index;
        *index = index.saturating_add(1);
        self.metadata.is_focused().then_some(current)
    }

    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        let focused = *index == target;
        self.metadata.set_focused(focused);
        if focused {
            self.metadata.request_scroll_into_view();
        } else {
            self.metadata.clear_scroll_into_view_request();
        }
        *index = index.saturating_add(1);
    }

    fn __focusable_index_at_position_inner(
        &self,
        column: u16,
        row: u16,
        index: &mut usize,
    ) -> Option<usize> {
        let current = *index;
        *index = index.saturating_add(1);
        self.metadata
            .contains_hit_position(column, row)
            .then_some(current)
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        (self.metadata.is_focused() && self.metadata.scroll_into_view_requested())
            .then_some((0, u32::from(ctx.area().height)))
    }

    fn __activate_focused_button(&self) -> Result<Option<AppControl>> {
        Ok(self.metadata.is_focused().then(|| {
            self.on_press
                .as_ref()
                .map_or(AppControl::Continue, |action| action())
        }))
    }

    fn __focused_control(&self) -> Option<FocusedControl> {
        self.metadata.is_focused().then_some(FocusedControl::Button)
    }
}

impl_styled_view!(ButtonView);
