//! Controlled multiline text-area view.

use std::{fmt, time::Instant};

use crossterm::event::KeyEvent;

use crate::view::{StyleMetadata, View, ViewType};
use crate::{
    app::{AppControl, Result},
    component::{FocusedControl, KeyControl, RenderCtx},
};

use super::{
    insert::{flush_expired_insert_key, handle_text_area_key, has_active_insert_key_pending},
    model::{EditableControlKind, EditableModel, impl_editable_view_api},
    render::{
        focused_control_span_for_editor, min_height_for_editable_text_view,
        render_editable_text_view,
    },
    state::{EditableState, VimMode},
};

/// Controlled multiline text area.
#[derive(PartialEq)]
pub struct TextAreaView {
    /// Shared editable storage configured for text-area behavior.
    pub(crate) model: EditableModel,
}

/// Creates a controlled multiline text area.
///
/// # Arguments
///
/// * `value` — Caller-owned value displayed by the text area.
///
/// # Returns
///
/// A [`TextAreaView`] with fresh editing state.
pub fn text_area(value: impl Into<String>) -> TextAreaView {
    let value = value.into();
    let mut editable_state = EditableState::new();
    editable_state.set_cursor(value.len());

    TextAreaView {
        model: EditableModel {
            value,
            placeholder: None,
            kind: EditableControlKind::TextArea,
            metadata: StyleMetadata::new(ViewType::TextArea),
            on_input: None,
            editable_state,
        },
    }
}

impl fmt::Debug for TextAreaView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextAreaView")
            .field("value", &self.model.value)
            .field("placeholder", &self.model.placeholder)
            .field("metadata", &self.model.metadata)
            .field("has_on_input", &self.model.on_input.is_some())
            .field("editable_state", &self.model.editable_state)
            .finish()
    }
}

impl View for TextAreaView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        render_editable_text_view(&self.model, ctx)
    }

    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        min_height_for_editable_text_view(&self.model, ctx)
    }

    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.model.metadata)
    }

    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.model.metadata)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn reconcile(&mut self, previous: &dyn View) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.model.editable_state = previous.model.editable_state.clone();
        }
    }

    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        flush_expired_insert_key(
            &self.model.value,
            &self.model.on_input,
            &mut self.model.editable_state,
            Instant::now(),
        )
    }

    fn __focusable_count(&self) -> usize {
        1
    }

    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        let current = *index;
        *index = index.saturating_add(1);
        self.model.metadata.is_focused().then_some(current)
    }

    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        let focused = *index == target;
        self.model.metadata.set_focused(focused);
        if focused {
            self.model.editable_state.set_mode(VimMode::Normal);
            self.model.metadata.request_scroll_into_view();
        } else {
            self.model.metadata.clear_scroll_into_view_request();
        }
        *index = index.saturating_add(1);
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        focused_control_span_for_editor(&self.model, ctx).map(|span| span.into_tuple())
    }

    fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        if !self.model.metadata.is_focused() {
            return None;
        }
        let control = handle_text_area_key(
            &self.model.value,
            &self.model.on_input,
            &mut self.model.editable_state,
            &key,
        );
        if matches!(control, Some(KeyControl::Handled | KeyControl::Exit)) {
            self.model.metadata.request_scroll_into_view();
        }
        control
    }

    fn __focused_control(&self) -> Option<FocusedControl> {
        if !self.model.metadata.is_focused() {
            return None;
        }
        let insert_mode = self.model.editable_state.mode() == VimMode::Insert
            && !has_active_insert_key_pending(&self.model.editable_state, Instant::now());
        let visual_mode = matches!(
            self.model.editable_state.mode(),
            VimMode::Visual | VimMode::VisualLine
        );
        Some(FocusedControl::TextArea {
            insert_mode,
            visual_mode,
        })
    }
}

impl_editable_view_api!(TextAreaView);
