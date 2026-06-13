//! Render-tree view data structures.
//!
//! This module defines the view enum and its equality/debug behavior used by
//! view builders and renderers.

use std::{fmt, rc::Rc};

use super::{
    component_view::ComponentView,
    dynamic::DynamicView,
    metadata::{StyleMetadata, ViewType},
};
use crate::app::AppControl;

/// Shared callback invoked when a button is activated.
pub type ButtonAction = Rc<dyn Fn() -> AppControl>;

/// Minimal renderable view tree for hand-written terminal UI.
#[derive(Clone)]
pub enum View {
    /// Bordered container around a child view.
    Block {
        /// Child view rendered inside the block's inner area.
        child: Box<View>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Plain text content.
    Text {
        /// Text content to render.
        content: String,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Horizontally arranged children.
    Row {
        /// Child views divided across the row.
        children: Vec<View>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Vertically arranged children.
    Column {
        /// Child views divided down the column.
        children: Vec<View>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Basic bordered button label.
    Button {
        /// Button label to render.
        label: String,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
        /// Optional activation callback.
        on_press: Option<ButtonAction>,
    },
    /// Child view produced when the tree is traversed.
    Dynamic(DynamicView),
    /// Child component preserved as a tree boundary.
    Component(ComponentView),
}

impl View {
    /// Returns selector metadata for styleable static views.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing a [`StyleMetadata`] reference for views that
    /// carry selector metadata.
    pub fn style_metadata(&self) -> Option<&StyleMetadata> {
        match self {
            Self::Block { metadata, .. }
            | Self::Text { metadata, .. }
            | Self::Row { metadata, .. }
            | Self::Column { metadata, .. }
            | Self::Button { metadata, .. } => Some(metadata),
            Self::Dynamic(_) | Self::Component(_) => None,
        }
    }

    /// Returns mutable selector metadata for styleable static views.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing a mutable [`StyleMetadata`] reference for views
    /// that carry selector metadata.
    pub fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        match self {
            Self::Block { metadata, .. }
            | Self::Text { metadata, .. }
            | Self::Row { metadata, .. }
            | Self::Column { metadata, .. }
            | Self::Button { metadata, .. } => Some(metadata),
            Self::Dynamic(_) | Self::Component(_) => None,
        }
    }

    /// Sets an id selector value on a styleable view.
    ///
    /// # Arguments
    ///
    /// * `id` — Id selector value to store.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with the provided id when the view is styleable.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_id(id);
        }

        self
    }

    /// Sets class selector values on a styleable view.
    ///
    /// # Arguments
    ///
    /// * `classes` — Whitespace-separated class selector values to store.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with the provided classes when the view is styleable.
    pub fn with_classes(mut self, classes: impl Into<String>) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_classes(classes);
        }

        self
    }

    /// Sets an inline style override on a styleable view.
    ///
    /// # Arguments
    ///
    /// * `style` — Inline style override to store.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with the provided inline style when the view is
    /// styleable.
    pub fn with_inline_style(mut self, style: crate::style::TuiStyle) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_inline_style(style);
        }

        self
    }

    /// Sets the current focus pseudo-class state on a styleable view.
    ///
    /// # Arguments
    ///
    /// * `focused` — Whether the view should match `:focus`.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with the provided focus state when the view is
    /// styleable.
    pub fn with_focus(mut self, focused: bool) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_focused(focused);
        }

        self
    }

    /// Stores an activation callback on a button view.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback invoked when this button is focused and activated.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with the callback when the view is a button.
    pub fn on_press(mut self, action: impl Fn() -> AppControl + 'static) -> Self {
        if let Self::Button { on_press, .. } = &mut self {
            *on_press = Some(Rc::new(action));
        }

        self
    }
}

impl fmt::Debug for View {
    /// Formats a view tree for diagnostics.
    ///
    /// Dynamic views avoid formatting their closures because closures do not
    /// implement [`fmt::Debug`].
    ///
    /// # Arguments
    ///
    /// * `f` — Formatter receiving the debug representation.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating whether formatting succeeded.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block { child, metadata } => f
                .debug_struct("Block")
                .field("child", child)
                .field("metadata", metadata)
                .finish(),
            Self::Text { content, metadata } => f
                .debug_struct("Text")
                .field("content", content)
                .field("metadata", metadata)
                .finish(),
            Self::Row { children, metadata } => f
                .debug_struct("Row")
                .field("children", children)
                .field("metadata", metadata)
                .finish(),
            Self::Column { children, metadata } => f
                .debug_struct("Column")
                .field("children", children)
                .field("metadata", metadata)
                .finish(),
            Self::Button {
                label,
                metadata,
                on_press,
            } => f
                .debug_struct("Button")
                .field("label", label)
                .field("metadata", metadata)
                .field("on_press", &on_press.is_some())
                .finish(),
            Self::Dynamic(_) => f.write_str("Dynamic(..)"),
            Self::Component(component) => f.debug_tuple("Component").field(component).finish(),
        }
    }
}

/// Returns whether optional button actions represent the same callback.
///
/// # Arguments
///
/// * `left` — Left optional button action to compare.
/// * `right` — Right optional button action to compare.
///
/// # Returns
///
/// A [`bool`] indicating whether both callbacks are absent or share identity.
fn button_actions_equal(left: &Option<ButtonAction>, right: &Option<ButtonAction>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
        _ => false,
    }
}

impl PartialEq for View {
    /// Compares view trees by value, using pointer identity for deferred views.
    ///
    /// # Arguments
    ///
    /// * `other` — View to compare with `self`.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether the views are equal.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Block {
                    child: left_child,
                    metadata: left_metadata,
                },
                Self::Block {
                    child: right_child,
                    metadata: right_metadata,
                },
            ) => left_child == right_child && left_metadata == right_metadata,
            (
                Self::Text {
                    content: left_content,
                    metadata: left_metadata,
                },
                Self::Text {
                    content: right_content,
                    metadata: right_metadata,
                },
            ) => left_content == right_content && left_metadata == right_metadata,
            (
                Self::Row {
                    children: left_children,
                    metadata: left_metadata,
                },
                Self::Row {
                    children: right_children,
                    metadata: right_metadata,
                },
            ) => left_children == right_children && left_metadata == right_metadata,
            (
                Self::Column {
                    children: left_children,
                    metadata: left_metadata,
                },
                Self::Column {
                    children: right_children,
                    metadata: right_metadata,
                },
            ) => left_children == right_children && left_metadata == right_metadata,
            (
                Self::Button {
                    label: left_label,
                    metadata: left_metadata,
                    on_press: left_on_press,
                },
                Self::Button {
                    label: right_label,
                    metadata: right_metadata,
                    on_press: right_on_press,
                },
            ) => {
                left_label == right_label
                    && left_metadata == right_metadata
                    && button_actions_equal(left_on_press, right_on_press)
            }
            (Self::Dynamic(left), Self::Dynamic(right)) => Rc::ptr_eq(left, right),
            (Self::Component(left), Self::Component(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

impl Eq for View {}

impl From<String> for View {
    /// Converts owned text into a text view.
    ///
    /// # Arguments
    ///
    /// * `value` — Text content to render.
    ///
    /// # Returns
    ///
    /// A [`View::Text`] containing `value`.
    fn from(value: String) -> Self {
        Self::Text {
            content: value,
            metadata: StyleMetadata::new(ViewType::Text),
        }
    }
}

impl From<&str> for View {
    /// Converts borrowed text into a text view.
    ///
    /// # Arguments
    ///
    /// * `value` — Text content to copy into the view.
    ///
    /// # Returns
    ///
    /// A [`View::Text`] containing `value`.
    fn from(value: &str) -> Self {
        Self::Text {
            content: value.to_owned(),
            metadata: StyleMetadata::new(ViewType::Text),
        }
    }
}
