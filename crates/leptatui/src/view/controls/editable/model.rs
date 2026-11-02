//! Shared storage and internal geometry for editable controls.

use std::rc::Rc;

use crate::{app::AppControl, view::StyleMetadata};

use super::state::{EditableState, VimMode};

/// Shared callback invoked when an editable control proposes a new value.
pub type EditableAction = Rc<dyn Fn(String) -> AppControl>;

/// Editing geometry selected by a concrete editable view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EditableControlKind {
    /// Single-line horizontally scrolling input.
    Input,
    /// Multiline vertically scrolling text area.
    TextArea,
}

/// Storage shared by input and text-area views.
pub(crate) struct EditableModel {
    /// Caller-owned value displayed by the editor.
    pub(crate) value: String,
    /// Placeholder displayed for an empty value.
    pub(crate) placeholder: Option<String>,
    /// Editing geometry selected by the owning concrete view.
    pub(crate) kind: EditableControlKind,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
    /// Optional controlled-value callback.
    pub(crate) on_input: Option<EditableAction>,
    /// Retained cursor, selection, scroll, and history state.
    pub(crate) editable_state: EditableState,
}

impl EditableModel {
    /// Synchronizes retained editing mode into stylesheet metadata.
    pub(crate) fn sync_style_state(&self) {
        let mode = self.editable_state.mode();
        self.metadata.sync_insert(mode == VimMode::Insert);
        self.metadata
            .sync_visual(matches!(mode, VimMode::Visual | VimMode::VisualLine));
    }
}

impl PartialEq for EditableModel {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.placeholder == other.placeholder
            && self.kind == other.kind
            && self.metadata == other.metadata
            && super::super::actions_equal(&self.on_input, &other.on_input)
            && self.editable_state == other.editable_state
    }
}

/// Implements shared fluent configuration and accessors for an editable view.
macro_rules! impl_editable_view_api {
    ($type:ty) => {
        impl $crate::view::StyledView for $type {
            fn metadata(&self) -> &$crate::view::StyleMetadata {
                self.model.sync_style_state();
                &self.model.metadata
            }

            fn metadata_mut(&mut self) -> &mut $crate::view::StyleMetadata {
                &mut self.model.metadata
            }
        }

        impl $crate::view::EditableView for $type {
            fn __placeholder_mut(&mut self) -> &mut Option<String> {
                &mut self.model.placeholder
            }

            fn __on_input_mut(&mut self) -> &mut Option<$crate::view::EditableAction> {
                &mut self.model.on_input
            }
        }

        impl $type {
            /// Returns this view's selector and runtime metadata.
            pub fn metadata(&self) -> &$crate::view::StyleMetadata {
                $crate::view::StyledView::metadata(self)
            }

            /// Returns mutable selector and runtime metadata.
            pub fn metadata_mut(&mut self) -> &mut $crate::view::StyleMetadata {
                $crate::view::StyledView::metadata_mut(self)
            }

            /// Returns selector metadata through the core view terminology.
            pub fn style_metadata(&self) -> Option<&$crate::view::StyleMetadata> {
                Some($crate::view::StyledView::metadata(self))
            }

            /// Returns mutable selector metadata through the core view terminology.
            pub fn style_metadata_mut(&mut self) -> Option<&mut $crate::view::StyleMetadata> {
                Some($crate::view::StyledView::metadata_mut(self))
            }

            /// Sets an id selector value.
            pub fn with_id(self, id: impl Into<String>) -> Self {
                $crate::view::StyledView::with_id(self, id)
            }

            /// Sets whitespace-separated class selector values.
            pub fn with_classes(self, classes: impl Into<String>) -> Self {
                $crate::view::StyledView::with_classes(self, classes)
            }

            /// Sets an inline style override.
            pub fn with_inline_style(self, style: $crate::TuiStyle) -> Self {
                $crate::view::StyledView::with_inline_style(self, style)
            }

            /// Sets the current focus pseudo-class state.
            pub fn with_focus(self, focused: bool) -> Self {
                $crate::view::StyledView::with_focus(self, focused)
            }

            /// Sets the current insert pseudo-class state.
            pub fn with_insert(self, insert: bool) -> Self {
                $crate::view::StyledView::with_insert(self, insert)
            }

            /// Sets the current visual pseudo-class state.
            ///
            /// # Arguments
            ///
            /// * `visual` — Whether this view should match the visual selector.
            ///
            /// # Returns
            ///
            /// This view with the updated visual state.
            pub fn with_visual(self, visual: bool) -> Self {
                $crate::view::StyledView::with_visual(self, visual)
            }

            /// Sets the current visited pseudo-class state.
            ///
            /// # Arguments
            ///
            /// * `visited` — Whether this view should match the visited selector.
            ///
            /// # Returns
            ///
            /// This view with the updated visited state.
            pub fn with_visited(self, visited: bool) -> Self {
                $crate::view::StyledView::with_visited(self, visited)
            }

            /// Stores placeholder text.
            pub fn placeholder(self, placeholder: impl Into<String>) -> Self {
                $crate::view::EditableView::placeholder(self, placeholder)
            }

            /// Stores a callback invoked with proposed controlled values.
            pub fn on_input(self, action: impl Fn(String) -> $crate::AppControl + 'static) -> Self {
                $crate::view::EditableView::on_input(self, action)
            }

            /// Returns the controlled text value.
            pub fn value(&self) -> &str {
                &self.model.value
            }

            /// Returns optional placeholder text.
            pub fn placeholder_text(&self) -> Option<&str> {
                self.model.placeholder.as_deref()
            }

            /// Returns retained cursor, selection, scrolling, and history state.
            pub fn editable_state(&self) -> &$crate::view::EditableState {
                &self.model.editable_state
            }

            /// Returns mutable retained cursor, selection, scrolling, and history state.
            pub fn editable_state_mut(&mut self) -> &mut $crate::view::EditableState {
                &mut self.model.editable_state
            }

            /// Returns whether a controlled-value callback is configured.
            pub fn has_on_input(&self) -> bool {
                self.model.on_input.is_some()
            }
        }
    };
}
pub(crate) use impl_editable_view_api;
