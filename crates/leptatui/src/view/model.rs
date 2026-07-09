//! Render-tree view data structures.
//!
//! This module defines the view enum and its equality/debug behavior used by
//! view builders and renderers.

use std::{fmt, path::PathBuf, rc::Rc};

use ratatui::text::Text;

use super::{
    component_view::ComponentView,
    dynamic::DynamicView,
    metadata::{EditableState, StyleMetadata, ViewType},
};
use crate::app::AppControl;

/// Shared callback invoked when a button is activated.
pub type ButtonAction = Rc<dyn Fn() -> AppControl>;

/// Shared callback invoked when a form is submitted or canceled.
pub type FormAction = Rc<dyn Fn() -> AppControl>;

/// Shared callback invoked when an input proposes a new value.
pub type InputAction = Rc<dyn Fn(String) -> AppControl>;

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

/// Returns a clamped progress value safe for Ratatui gauge rendering.
pub(crate) fn clamped_progress_value(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

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
    /// First-level semantic heading content.
    H1 {
        /// Rich text content to render.
        content: Text<'static>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Second-level semantic heading content.
    H2 {
        /// Rich text content to render.
        content: Text<'static>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Third-level semantic heading content.
    H3 {
        /// Rich text content to render.
        content: Text<'static>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Fourth-level semantic heading content.
    H4 {
        /// Rich text content to render.
        content: Text<'static>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Fifth-level semantic heading content.
    H5 {
        /// Rich text content to render.
        content: Text<'static>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Sixth-level semantic heading content.
    H6 {
        /// Rich text content to render.
        content: Text<'static>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Semantic paragraph content.
    Paragraph {
        /// Rich text content to render.
        content: Text<'static>,
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
    /// Grouping container for controls and submit/cancel behavior.
    Form {
        /// Child views grouped inside the form.
        children: Vec<View>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
        /// Optional submit callback.
        on_submit: Option<FormAction>,
        /// Optional cancel callback.
        on_cancel: Option<FormAction>,
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
    /// Preparatory single-line editable text control.
    Input {
        /// Caller-owned value to display.
        value: String,
        /// Placeholder text shown when the value is empty.
        placeholder: Option<String>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
        /// Optional controlled-value change callback.
        on_input: Option<InputAction>,
        /// Retained editing state for reconciled redraws.
        editable_state: EditableState,
    },
    /// Preparatory multiline editable text control.
    TextArea {
        /// Caller-owned value to display.
        value: String,
        /// Placeholder text shown when the value is empty.
        placeholder: Option<String>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
        /// Optional controlled-value change callback.
        on_input: Option<InputAction>,
        /// Retained editing state for reconciled redraws.
        editable_state: EditableState,
    },
    /// Path-backed terminal image with deterministic text fallback.
    Image {
        /// Image source to render.
        source: ImageSource,
        /// Fallback text rendered when image support is unavailable.
        alt: Option<String>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
    },
    /// Gauge-style progress indicator.
    ProgressBar {
        /// Completion ratio requested by the caller.
        value: f64,
        /// Optional label rendered over the gauge.
        label: Option<String>,
        /// Selector metadata for matching this view.
        metadata: StyleMetadata,
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
            | Self::H1 { metadata, .. }
            | Self::H2 { metadata, .. }
            | Self::H3 { metadata, .. }
            | Self::H4 { metadata, .. }
            | Self::H5 { metadata, .. }
            | Self::H6 { metadata, .. }
            | Self::Paragraph { metadata, .. }
            | Self::Row { metadata, .. }
            | Self::Column { metadata, .. }
            | Self::Form { metadata, .. }
            | Self::Button { metadata, .. }
            | Self::Input { metadata, .. }
            | Self::TextArea { metadata, .. }
            | Self::Image { metadata, .. }
            | Self::ProgressBar { metadata, .. } => Some(metadata),
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
            | Self::H1 { metadata, .. }
            | Self::H2 { metadata, .. }
            | Self::H3 { metadata, .. }
            | Self::H4 { metadata, .. }
            | Self::H5 { metadata, .. }
            | Self::H6 { metadata, .. }
            | Self::Paragraph { metadata, .. }
            | Self::Row { metadata, .. }
            | Self::Column { metadata, .. }
            | Self::Form { metadata, .. }
            | Self::Button { metadata, .. }
            | Self::Input { metadata, .. }
            | Self::TextArea { metadata, .. }
            | Self::Image { metadata, .. }
            | Self::ProgressBar { metadata, .. } => Some(metadata),
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

    /// Stores a submit callback on a form view.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback invoked when a focused descendant submits the
    ///   form.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with the callback when the view is a form.
    pub fn on_submit(mut self, action: impl Fn() -> AppControl + 'static) -> Self {
        if let Self::Form { on_submit, .. } = &mut self {
            *on_submit = Some(Rc::new(action));
        }

        self
    }

    /// Stores a cancel callback on a form view.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback invoked when a focused descendant cancels the form.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with the callback when the view is a form.
    pub fn on_cancel(mut self, action: impl Fn() -> AppControl + 'static) -> Self {
        if let Self::Form { on_cancel, .. } = &mut self {
            *on_cancel = Some(Rc::new(action));
        }

        self
    }

    /// Stores placeholder text on an editable text control.
    ///
    /// # Arguments
    ///
    /// * `placeholder` — Text displayed when the input value is empty.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with placeholder text when the view is an editable
    /// text control.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        match &mut self {
            Self::Input {
                placeholder: slot, ..
            }
            | Self::TextArea {
                placeholder: slot, ..
            } => {
                *slot = Some(placeholder.into());
            }
            _ => {}
        }

        self
    }

    /// Stores a controlled-value callback on an editable text control.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback invoked with the next value when editing keys are
    ///   pressed while this control is focused.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with the callback when the view is an editable text
    /// control.
    pub fn on_input(mut self, action: impl Fn(String) -> AppControl + 'static) -> Self {
        match &mut self {
            Self::Input { on_input, .. } | Self::TextArea { on_input, .. } => {
                *on_input = Some(Rc::new(action));
            }
            _ => {}
        }

        self
    }

    /// Stores fallback text on an image view.
    ///
    /// # Arguments
    ///
    /// * `alt` — Text displayed when image rendering is unavailable or fails.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with fallback text when the view is an image.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        if let Self::Image { alt: slot, .. } = &mut self {
            *slot = Some(alt.into());
        }

        self
    }

    /// Stores label text on a progress bar view.
    ///
    /// # Arguments
    ///
    /// * `label` — Text rendered over the progress bar gauge.
    ///
    /// # Returns
    ///
    /// A [`View`] updated with label text when the view is a progress bar.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        if let Self::ProgressBar { label: slot, .. } = &mut self {
            *slot = Some(label.into());
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
            Self::H1 { content, metadata } => f
                .debug_struct("H1")
                .field("content", content)
                .field("metadata", metadata)
                .finish(),
            Self::H2 { content, metadata } => f
                .debug_struct("H2")
                .field("content", content)
                .field("metadata", metadata)
                .finish(),
            Self::H3 { content, metadata } => f
                .debug_struct("H3")
                .field("content", content)
                .field("metadata", metadata)
                .finish(),
            Self::H4 { content, metadata } => f
                .debug_struct("H4")
                .field("content", content)
                .field("metadata", metadata)
                .finish(),
            Self::H5 { content, metadata } => f
                .debug_struct("H5")
                .field("content", content)
                .field("metadata", metadata)
                .finish(),
            Self::H6 { content, metadata } => f
                .debug_struct("H6")
                .field("content", content)
                .field("metadata", metadata)
                .finish(),
            Self::Paragraph { content, metadata } => f
                .debug_struct("Paragraph")
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
            Self::Form {
                children,
                metadata,
                on_submit,
                on_cancel,
            } => f
                .debug_struct("Form")
                .field("children", children)
                .field("metadata", metadata)
                .field("on_submit", &on_submit.is_some())
                .field("on_cancel", &on_cancel.is_some())
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
            Self::Input {
                value,
                placeholder,
                metadata,
                on_input,
                editable_state,
            } => f
                .debug_struct("Input")
                .field("value", value)
                .field("placeholder", placeholder)
                .field("metadata", metadata)
                .field("on_input", &on_input.is_some())
                .field("editable_state", editable_state)
                .finish(),
            Self::TextArea {
                value,
                placeholder,
                metadata,
                on_input,
                editable_state,
            } => f
                .debug_struct("TextArea")
                .field("value", value)
                .field("placeholder", placeholder)
                .field("metadata", metadata)
                .field("on_input", &on_input.is_some())
                .field("editable_state", editable_state)
                .finish(),
            Self::Image {
                source,
                alt,
                metadata,
            } => f
                .debug_struct("Image")
                .field("source", source)
                .field("alt", alt)
                .field("metadata", metadata)
                .finish(),
            Self::ProgressBar {
                value,
                label,
                metadata,
            } => f
                .debug_struct("ProgressBar")
                .field("value", value)
                .field("label", label)
                .field("metadata", metadata)
                .finish(),
            Self::Dynamic(_) => f.write_str("Dynamic(..)"),
            Self::Component(component) => f.debug_tuple("Component").field(component).finish(),
        }
    }
}

/// Returns whether optional actions represent the same callback.
///
/// # Arguments
///
/// * `left` — Left optional action to compare.
/// * `right` — Right optional action to compare.
///
/// # Returns
///
/// A [`bool`] indicating whether both callbacks are absent or share identity.
fn actions_equal<T: ?Sized>(left: &Option<Rc<T>>, right: &Option<Rc<T>>) -> bool {
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
                Self::H1 {
                    content: left_content,
                    metadata: left_metadata,
                },
                Self::H1 {
                    content: right_content,
                    metadata: right_metadata,
                },
            )
            | (
                Self::H2 {
                    content: left_content,
                    metadata: left_metadata,
                },
                Self::H2 {
                    content: right_content,
                    metadata: right_metadata,
                },
            )
            | (
                Self::H3 {
                    content: left_content,
                    metadata: left_metadata,
                },
                Self::H3 {
                    content: right_content,
                    metadata: right_metadata,
                },
            )
            | (
                Self::H4 {
                    content: left_content,
                    metadata: left_metadata,
                },
                Self::H4 {
                    content: right_content,
                    metadata: right_metadata,
                },
            )
            | (
                Self::H5 {
                    content: left_content,
                    metadata: left_metadata,
                },
                Self::H5 {
                    content: right_content,
                    metadata: right_metadata,
                },
            )
            | (
                Self::H6 {
                    content: left_content,
                    metadata: left_metadata,
                },
                Self::H6 {
                    content: right_content,
                    metadata: right_metadata,
                },
            )
            | (
                Self::Paragraph {
                    content: left_content,
                    metadata: left_metadata,
                },
                Self::Paragraph {
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
                Self::Form {
                    children: left_children,
                    metadata: left_metadata,
                    on_submit: left_on_submit,
                    on_cancel: left_on_cancel,
                },
                Self::Form {
                    children: right_children,
                    metadata: right_metadata,
                    on_submit: right_on_submit,
                    on_cancel: right_on_cancel,
                },
            ) => {
                left_children == right_children
                    && left_metadata == right_metadata
                    && actions_equal(left_on_submit, right_on_submit)
                    && actions_equal(left_on_cancel, right_on_cancel)
            }
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
                    && actions_equal(left_on_press, right_on_press)
            }
            (
                Self::Input {
                    value: left_value,
                    placeholder: left_placeholder,
                    metadata: left_metadata,
                    on_input: left_on_input,
                    editable_state: left_editable_state,
                },
                Self::Input {
                    value: right_value,
                    placeholder: right_placeholder,
                    metadata: right_metadata,
                    on_input: right_on_input,
                    editable_state: right_editable_state,
                },
            ) => {
                left_value == right_value
                    && left_placeholder == right_placeholder
                    && left_metadata == right_metadata
                    && actions_equal(left_on_input, right_on_input)
                    && left_editable_state == right_editable_state
            }
            (
                Self::TextArea {
                    value: left_value,
                    placeholder: left_placeholder,
                    metadata: left_metadata,
                    on_input: left_on_input,
                    editable_state: left_editable_state,
                },
                Self::TextArea {
                    value: right_value,
                    placeholder: right_placeholder,
                    metadata: right_metadata,
                    on_input: right_on_input,
                    editable_state: right_editable_state,
                },
            ) => {
                left_value == right_value
                    && left_placeholder == right_placeholder
                    && left_metadata == right_metadata
                    && actions_equal(left_on_input, right_on_input)
                    && left_editable_state == right_editable_state
            }
            (
                Self::Image {
                    source: left_source,
                    alt: left_alt,
                    metadata: left_metadata,
                },
                Self::Image {
                    source: right_source,
                    alt: right_alt,
                    metadata: right_metadata,
                },
            ) => {
                left_source == right_source
                    && left_alt == right_alt
                    && left_metadata == right_metadata
            }
            (
                Self::ProgressBar {
                    value: left_value,
                    label: left_label,
                    metadata: left_metadata,
                },
                Self::ProgressBar {
                    value: right_value,
                    label: right_label,
                    metadata: right_metadata,
                },
            ) => {
                left_value.to_bits() == right_value.to_bits()
                    && left_label == right_label
                    && left_metadata == right_metadata
            }
            (Self::Dynamic(left), Self::Dynamic(right)) => left.ptr_eq(right),
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
