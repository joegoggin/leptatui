//! Optional capabilities implemented by concrete terminal views.

use std::rc::Rc;

use ratatui::text::Text as RichText;

use crate::{app::AppControl, style::TuiStyle};

use super::{any_view::AnyView, metadata::StyleMetadata};
use crate::view::controls::editable::model::EditableAction;

/// Fluent styling behavior shared by concrete styleable views.
pub trait StyledView: Sized {
    /// Returns this view's selector metadata.
    ///
    /// # Returns
    ///
    /// A shared [`StyleMetadata`] reference.
    fn metadata(&self) -> &StyleMetadata;

    /// Returns this view's mutable selector metadata.
    ///
    /// # Returns
    ///
    /// A mutable [`StyleMetadata`] reference.
    fn metadata_mut(&mut self) -> &mut StyleMetadata;

    /// Sets an id selector value.
    ///
    /// # Arguments
    ///
    /// * `id` — Id selector value to store.
    ///
    /// # Returns
    ///
    /// This view with the updated metadata.
    fn with_id(mut self, id: impl Into<String>) -> Self {
        self.metadata_mut().set_id(id);
        self
    }

    /// Sets whitespace-separated class selector values.
    ///
    /// # Arguments
    ///
    /// * `classes` — Whitespace-separated class selector values to store.
    ///
    /// # Returns
    ///
    /// This view with the updated metadata.
    fn with_classes(mut self, classes: impl Into<String>) -> Self {
        self.metadata_mut().set_classes(classes);
        self
    }

    /// Sets an inline style override.
    ///
    /// # Arguments
    ///
    /// * `style` — Inline style override to store.
    ///
    /// # Returns
    ///
    /// This view with the updated metadata.
    fn with_inline_style(mut self, style: TuiStyle) -> Self {
        self.metadata_mut().set_inline_style(style);
        self
    }

    /// Sets the current focus pseudo-class state.
    ///
    /// # Arguments
    ///
    /// * `focused` — Whether this view should match the focus selector.
    ///
    /// # Returns
    ///
    /// This view with the updated focus state.
    fn with_focus(mut self, focused: bool) -> Self {
        self.metadata_mut().set_focused(focused);
        self
    }

    /// Sets the current active pseudo-class state.
    ///
    /// # Arguments
    ///
    /// * `active` — Whether this view should match the active selector.
    ///
    /// # Returns
    ///
    /// This view with the updated active state.
    fn with_active(mut self, active: bool) -> Self {
        self.metadata_mut().set_active(active);
        self
    }

    /// Sets the current insert pseudo-class state.
    ///
    /// # Arguments
    ///
    /// * `insert` — Whether this view should match the insert selector.
    ///
    /// # Returns
    ///
    /// This view with the updated insert state.
    fn with_insert(mut self, insert: bool) -> Self {
        self.metadata_mut().set_insert(insert);
        self
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
    fn with_visual(mut self, visual: bool) -> Self {
        self.metadata_mut().set_visual(visual);
        self
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
    fn with_visited(mut self, visited: bool) -> Self {
        self.metadata_mut().set_visited(visited);
        self
    }
}

/// Child access shared by concrete container views.
pub trait ContainerView {
    /// Returns direct children in render order.
    ///
    /// # Returns
    ///
    /// A slice of type-erased child views.
    fn child_views(&self) -> &[AnyView];

    /// Returns mutable direct children in render order.
    ///
    /// # Returns
    ///
    /// A mutable slice of type-erased child views.
    fn child_views_mut(&mut self) -> &mut [AnyView];
}

/// Rich text access shared by semantic text nodes.
pub trait TextualView {
    /// Returns the node's rich text content.
    ///
    /// # Returns
    ///
    /// A shared reference to the retained rich text.
    fn content(&self) -> &RichText<'static>;
}

/// Fluent configuration shared by input and text-area views.
pub trait EditableView: Sized {
    /// Returns the mutable placeholder slot.
    #[doc(hidden)]
    fn __placeholder_mut(&mut self) -> &mut Option<String>;

    /// Returns the mutable controlled-value callback slot.
    #[doc(hidden)]
    fn __on_input_mut(&mut self) -> &mut Option<EditableAction>;

    /// Stores placeholder text.
    ///
    /// # Arguments
    ///
    /// * `placeholder` — Text displayed when the controlled value is empty.
    ///
    /// # Returns
    ///
    /// This editable view with the placeholder configured.
    fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        *self.__placeholder_mut() = Some(placeholder.into());
        self
    }

    /// Stores a callback invoked with proposed controlled values.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback receiving each proposed next value.
    ///
    /// # Returns
    ///
    /// This editable view with the callback configured.
    fn on_input(mut self, action: impl Fn(String) -> AppControl + 'static) -> Self {
        *self.__on_input_mut() = Some(Rc::new(action));
        self
    }
}

/// Implements styling and inherent styling accessors for a concrete view.
macro_rules! impl_styled_view {
    ($type:ty) => {
        impl $crate::view::StyledView for $type {
            fn metadata(&self) -> &$crate::view::StyleMetadata {
                &self.metadata
            }

            fn metadata_mut(&mut self) -> &mut $crate::view::StyleMetadata {
                &mut self.metadata
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

            /// Sets the current active pseudo-class state.
            pub fn with_active(self, active: bool) -> Self {
                $crate::view::StyledView::with_active(self, active)
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
        }
    };
}
pub(crate) use impl_styled_view;

/// Implements child access and inherent child accessors for a concrete view.
macro_rules! impl_container_view {
    ($type:ty) => {
        impl $crate::view::ContainerView for $type {
            fn child_views(&self) -> &[$crate::view::AnyView] {
                &self.children
            }

            fn child_views_mut(&mut self) -> &mut [$crate::view::AnyView] {
                &mut self.children
            }
        }

        impl $type {
            /// Returns direct children in render order.
            pub fn children(&self) -> &[$crate::view::AnyView] {
                $crate::view::ContainerView::child_views(self)
            }

            /// Returns mutable direct children in render order.
            pub fn children_mut(&mut self) -> &mut [$crate::view::AnyView] {
                $crate::view::ContainerView::child_views_mut(self)
            }
        }
    };
}
pub(crate) use impl_container_view;

/// Implements rich-text access and its inherent accessor for a concrete view.
macro_rules! impl_textual_view {
    ($type:ty) => {
        impl $crate::view::TextualView for $type {
            fn content(&self) -> &ratatui::text::Text<'static> {
                &self.content
            }
        }

        impl $type {
            /// Returns this view's rich text content.
            pub fn content(&self) -> &ratatui::text::Text<'static> {
                $crate::view::TextualView::content(self)
            }
        }
    };
}
pub(crate) use impl_textual_view;
