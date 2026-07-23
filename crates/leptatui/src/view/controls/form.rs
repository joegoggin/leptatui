//! Form container with submit and cancel actions.

use std::{fmt, rc::Rc};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::view::containers::layout::render::{
    focused_control_span_for_layout_view, min_height_for_layout_view, render_layout_view,
};
use crate::view::core::{
    capabilities::{impl_container_view, impl_styled_view},
    render::VerticalSpan,
};
use crate::view::{AnyView, IntoViews, StyleMetadata, View, ViewType};
use crate::{
    app::{AppControl, Result},
    component::{FocusedControl, KeyControl, RenderCtx},
    style::LayoutDirection,
};

/// Shared callback invoked when a form is submitted or canceled.
pub type FormAction = Rc<dyn Fn() -> AppControl>;

/// Column layout that owns form submit and cancel actions.
pub struct FormView {
    /// Form control children.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
    /// Optional submit callback.
    pub(crate) on_submit: Option<FormAction>,
    /// Optional cancel callback.
    pub(crate) on_cancel: Option<FormAction>,
}

impl FormView {
    /// Stores a submit callback.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback invoked when the form submits.
    ///
    /// # Returns
    ///
    /// This form with the submit callback configured.
    pub fn on_submit(mut self, action: impl Fn() -> AppControl + 'static) -> Self {
        self.on_submit = Some(Rc::new(action));
        self
    }

    /// Stores a cancel callback.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback invoked when the form cancels.
    ///
    /// # Returns
    ///
    /// This form with the cancel callback configured.
    pub fn on_cancel(mut self, action: impl Fn() -> AppControl + 'static) -> Self {
        self.on_cancel = Some(Rc::new(action));
        self
    }

    /// Returns whether a submit callback is configured.
    ///
    /// # Returns
    ///
    /// `true` when this form has a submit callback.
    pub fn has_on_submit(&self) -> bool {
        self.on_submit.is_some()
    }

    /// Returns whether a cancel callback is configured.
    ///
    /// # Returns
    ///
    /// `true` when this form has a cancel callback.
    pub fn has_on_cancel(&self) -> bool {
        self.on_cancel.is_some()
    }
}

/// Creates a form container.
///
/// # Arguments
///
/// * `children` — Form controls and supporting child views.
///
/// # Returns
///
/// A [`FormView`] with no submit or cancel callbacks.
pub fn form(children: impl IntoViews) -> FormView {
    FormView {
        children: children.into_views(),
        metadata: StyleMetadata::new(ViewType::Form),
        on_submit: None,
        on_cancel: None,
    }
}

impl PartialEq for FormView {
    fn eq(&self, other: &Self) -> bool {
        self.children == other.children
            && self.metadata == other.metadata
            && super::actions_equal(&self.on_submit, &other.on_submit)
            && super::actions_equal(&self.on_cancel, &other.on_cancel)
    }
}

impl fmt::Debug for FormView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FormView")
            .field("children", &self.children)
            .field("metadata", &self.metadata)
            .field("has_on_submit", &self.on_submit.is_some())
            .field("has_on_cancel", &self.on_cancel.is_some())
            .finish()
    }
}

fn handle_form_focused_key(
    focused: FocusedControl,
    key: &KeyEvent,
    on_submit: &Option<FormAction>,
    on_cancel: &Option<FormAction>,
) -> Option<KeyControl> {
    match (focused, key.code) {
        (FocusedControl::Input { .. }, KeyCode::Enter) => Some(form_action_control(on_submit)),
        (FocusedControl::TextArea { .. }, KeyCode::Enter)
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Some(form_action_control(on_submit))
        }
        (
            FocusedControl::Input {
                insert_mode: true, ..
            },
            KeyCode::Esc,
        )
        | (
            FocusedControl::Input {
                visual_mode: true, ..
            },
            KeyCode::Esc,
        )
        | (
            FocusedControl::TextArea {
                insert_mode: true, ..
            },
            KeyCode::Esc,
        )
        | (
            FocusedControl::TextArea {
                visual_mode: true, ..
            },
            KeyCode::Esc,
        ) => None,
        (_, KeyCode::Esc) => Some(form_action_control(on_cancel)),
        _ => None,
    }
}

/// Converts an optional form action into key traversal control.
///
/// # Arguments
///
/// * `action` — Optional form callback to invoke.
///
/// # Returns
///
/// A [`KeyControl`] value that stops key propagation and reflects the callback
/// result.
fn form_action_control(action: &Option<FormAction>) -> KeyControl {
    action
        .as_ref()
        .map_or(KeyControl::Handled, |action| action().into())
}

impl View for FormView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        render_layout_view(&self.children, &self.metadata, LayoutDirection::Column, ctx)
    }

    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        min_height_for_layout_view(&self.children, &self.metadata, LayoutDirection::Column, ctx)
    }

    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }
    fn children(&self) -> &[AnyView] {
        &self.children
    }
    fn children_mut(&mut self) -> &mut [AnyView] {
        &mut self.children
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        if let Some(control) = self
            .children
            .iter_mut()
            .find_map(|child| child.__handle_form_key(key))
        {
            return Some(control);
        }
        let focused = self.children.iter().find_map(AnyView::__focused_control)?;
        handle_form_focused_key(focused, &key, &self.on_submit, &self.on_cancel)
    }

    fn __scroll_first_overflowing(&mut self, delta: i16) -> bool {
        if self.metadata.max_scroll_offset() > 0 && self.metadata.scroll_by(delta) {
            return true;
        }
        self.children
            .iter_mut()
            .any(|child| child.__scroll_first_overflowing(delta))
    }

    fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        if self.metadata.max_scroll_offset() > 0 && self.metadata.scroll_offset() > 0 {
            self.metadata.set_scroll_offset(0);
            return true;
        }
        self.children
            .iter_mut()
            .any(AnyView::__scroll_first_overflowing_to_top)
    }

    fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        let max = self.metadata.max_scroll_offset();
        if max > 0 && self.metadata.scroll_offset() < max {
            self.metadata.set_scroll_offset(max);
            return true;
        }
        self.children
            .iter_mut()
            .any(AnyView::__scroll_first_overflowing_to_bottom)
    }

    fn __has_overflowing_scroll_target(&self) -> bool {
        self.metadata.max_scroll_offset() > 0
            || self
                .children
                .iter()
                .any(AnyView::__has_overflowing_scroll_target)
    }

    fn __scroll_overflowing_at_position(&mut self, column: u16, row: u16, delta: i16) -> bool {
        if self
            .children
            .iter_mut()
            .any(|child| child.__scroll_overflowing_at_position(column, row, delta))
        {
            return true;
        }
        if self.metadata.max_scroll_offset() > 0 && self.metadata.contains_hit_position(column, row)
        {
            return self.metadata.scroll_by(delta);
        }
        false
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        focused_control_span_for_layout_view(
            &self.children,
            &self.metadata,
            LayoutDirection::Column,
            ctx,
        )
        .map(VerticalSpan::into_tuple)
    }
}

impl_styled_view!(FormView);
impl_container_view!(FormView);
