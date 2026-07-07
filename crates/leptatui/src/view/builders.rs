//! Convenience constructors for render-tree views.
//!
//! This module provides the public helper functions re-exported by
//! [`mod@crate::view`] and [`crate::prelude`].

use crate::component::Component;

use super::{
    component_view::ComponentView,
    metadata::{EditableState, StyleMetadata, ViewType},
    model::View,
};

/// Creates a bordered block around a child view.
///
/// # Arguments
///
/// * `child` — View-compatible value rendered inside the block.
///
/// # Returns
///
/// A [`View::Block`] containing the provided child.
pub fn block(child: impl Into<View>) -> View {
    View::Block {
        child: Box::new(child.into()),
        metadata: StyleMetadata::new(ViewType::Block),
    }
}

/// Creates a text view.
///
/// # Arguments
///
/// * `content` — Text content to render.
///
/// # Returns
///
/// A [`View::Text`] containing the provided content.
pub fn text(content: impl Into<String>) -> View {
    View::Text {
        content: content.into(),
        metadata: StyleMetadata::new(ViewType::Text),
    }
}

/// Creates a horizontal row.
///
/// # Arguments
///
/// * `children` — Child views to divide across the row.
///
/// # Returns
///
/// A [`View::Row`] containing the provided children.
pub fn row(children: impl IntoIterator<Item = View>) -> View {
    View::Row {
        children: children.into_iter().collect(),
        metadata: StyleMetadata::new(ViewType::Row),
    }
}

/// Creates a vertical column.
///
/// # Arguments
///
/// * `children` — Child views to divide down the column.
///
/// # Returns
///
/// A [`View::Column`] containing the provided children.
pub fn column(children: impl IntoIterator<Item = View>) -> View {
    View::Column {
        children: children.into_iter().collect(),
        metadata: StyleMetadata::new(ViewType::Column),
    }
}

/// Creates a form container.
///
/// # Arguments
///
/// * `children` — Child views grouped by the form.
///
/// # Returns
///
/// A [`View::Form`] containing the provided children.
pub fn form(children: impl IntoIterator<Item = View>) -> View {
    View::Form {
        children: children.into_iter().collect(),
        metadata: StyleMetadata::new(ViewType::Form),
        on_submit: None,
        on_cancel: None,
    }
}

/// Creates a basic button.
///
/// # Arguments
///
/// * `label` — Button text to center inside a bordered area.
///
/// # Returns
///
/// A [`View::Button`] containing the provided label.
pub fn button(label: impl Into<String>) -> View {
    View::Button {
        label: label.into(),
        metadata: StyleMetadata::new(ViewType::Button),
        on_press: None,
    }
}

/// Creates a controlled single-line input.
///
/// # Arguments
///
/// * `value` — Caller-owned value displayed by the input.
///
/// # Returns
///
/// A [`View::Input`] containing the provided value.
pub fn input(value: impl Into<String>) -> View {
    let value = value.into();
    let mut editable_state = EditableState::new();
    editable_state.set_cursor(value.len());

    View::Input {
        value,
        placeholder: None,
        metadata: StyleMetadata::new(ViewType::Input),
        on_input: None,
        editable_state,
    }
}

/// Creates a controlled multiline text area.
///
/// # Arguments
///
/// * `value` — Caller-owned value displayed by the text area.
///
/// # Returns
///
/// A [`View::TextArea`] containing the provided value.
pub fn text_area(value: impl Into<String>) -> View {
    let value = value.into();
    let mut editable_state = EditableState::new();
    editable_state.set_cursor(value.len());

    View::TextArea {
        value,
        placeholder: None,
        metadata: StyleMetadata::new(ViewType::TextArea),
        on_input: None,
        editable_state,
    }
}

/// Creates a dynamic child view.
///
/// # Arguments
///
/// * `child` — Closure that produces a view during render-tree traversal.
///
/// # Returns
///
/// A [`View::Dynamic`] containing the provided child closure.
pub fn dynamic(child: impl Fn() -> View + 'static) -> View {
    View::Dynamic(super::dynamic::DynamicView::new(child))
}

/// Creates a component-boundary view.
///
/// # Arguments
///
/// * `component` — Component value to preserve as a render-tree boundary.
///
/// # Returns
///
/// A [`View::Component`] containing the provided component.
pub fn component(component: impl Component + 'static) -> View {
    View::Component(ComponentView::new(component))
}

/// Creates a lazy component-boundary view from a component constructor.
pub(crate) fn component_factory<C>(
    preserve_on_reconcile: bool,
    factory: impl FnOnce() -> C + 'static,
) -> View
where
    C: Component + 'static,
{
    View::Component(ComponentView::new_factory(preserve_on_reconcile, factory))
}
