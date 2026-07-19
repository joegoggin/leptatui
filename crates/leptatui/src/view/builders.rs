//! Convenience constructors for render-tree views.
//!
//! This module provides the public helper functions re-exported by
//! [`mod@crate::view`] and [`crate::prelude`].

use ratatui::text::Text;

use crate::component::Component;

use super::{
    code_block::{SyntaxTheme, highlighted_source_lines},
    component_view::ComponentView,
    metadata::{EditableState, StyleMetadata, ViewType},
    model::{CellAlignment, ImageSource, View, clamped_progress_value},
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

/// Creates a first-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`View::H1`] containing the provided content.
pub fn h1(content: impl Into<Text<'static>>) -> View {
    View::H1 {
        content: content.into(),
        metadata: StyleMetadata::new(ViewType::H1),
    }
}

/// Creates a second-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`View::H2`] containing the provided content.
pub fn h2(content: impl Into<Text<'static>>) -> View {
    View::H2 {
        content: content.into(),
        metadata: StyleMetadata::new(ViewType::H2),
    }
}

/// Creates a third-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`View::H3`] containing the provided content.
pub fn h3(content: impl Into<Text<'static>>) -> View {
    View::H3 {
        content: content.into(),
        metadata: StyleMetadata::new(ViewType::H3),
    }
}

/// Creates a fourth-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`View::H4`] containing the provided content.
pub fn h4(content: impl Into<Text<'static>>) -> View {
    View::H4 {
        content: content.into(),
        metadata: StyleMetadata::new(ViewType::H4),
    }
}

/// Creates a fifth-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`View::H5`] containing the provided content.
pub fn h5(content: impl Into<Text<'static>>) -> View {
    View::H5 {
        content: content.into(),
        metadata: StyleMetadata::new(ViewType::H5),
    }
}

/// Creates a sixth-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`View::H6`] containing the provided content.
pub fn h6(content: impl Into<Text<'static>>) -> View {
    View::H6 {
        content: content.into(),
        metadata: StyleMetadata::new(ViewType::H6),
    }
}

/// Creates a semantic paragraph.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`View::Paragraph`] containing the provided content.
pub fn paragraph(content: impl Into<Text<'static>>) -> View {
    View::Paragraph {
        content: content.into(),
        metadata: StyleMetadata::new(ViewType::Paragraph),
    }
}

/// Creates a bordered syntax-highlighted code block.
///
/// The source is retained as logical lines. Supplying a recognized language
/// later through [`View::language`] highlights those lines once rather than on
/// every render frame. Unknown language tokens retain plain source. Logical
/// lines wrap to the available width rather than scrolling horizontally. The
/// selected syntax theme fills the block interior unless an authored
/// code-block background overrides it.
///
/// # Arguments
///
/// * `source` — Source code displayed inside the block.
///
/// # Returns
///
/// A [`View::CodeBlock`] using the dark theme with line numbers disabled.
pub fn code_block(source: impl Into<String>) -> View {
    let source = source.into();
    View::CodeBlock {
        highlighted_lines: highlighted_source_lines(&source, None, SyntaxTheme::Dark),
        source,
        language: None,
        line_numbers: false,
        syntax_theme: SyntaxTheme::Dark,
        metadata: StyleMetadata::new(ViewType::CodeBlock),
    }
}

/// Creates a semantic ordered list.
///
/// # Arguments
///
/// * `items` — List-item views to number from one.
///
/// # Returns
///
/// A [`View::OrderedList`] containing the provided items.
pub fn ordered_list(items: impl IntoIterator<Item = View>) -> View {
    View::OrderedList {
        items: items.into_iter().collect(),
        start: 1,
        metadata: StyleMetadata::new(ViewType::OrderedList),
    }
}

/// Creates a semantic unordered list.
///
/// # Arguments
///
/// * `items` — List-item views to mark with hyphens.
///
/// # Returns
///
/// A [`View::UnorderedList`] containing the provided items.
pub fn unordered_list(items: impl IntoIterator<Item = View>) -> View {
    View::UnorderedList {
        items: items.into_iter().collect(),
        metadata: StyleMetadata::new(ViewType::UnorderedList),
    }
}

/// Creates a semantic list item containing vertically stacked document blocks.
///
/// # Arguments
///
/// * `children` — Document blocks contained by the list item.
///
/// # Returns
///
/// A [`View::ListItem`] containing the provided children.
pub fn list_item(children: impl IntoIterator<Item = View>) -> View {
    View::ListItem {
        children: children.into_iter().collect(),
        metadata: StyleMetadata::new(ViewType::ListItem),
    }
}

/// Creates a semantic table from header and body sections.
///
/// # Arguments
///
/// * `sections` — Table-head and table-body views rendered in source order.
///
/// # Returns
///
/// A [`View::Table`] containing the provided sections.
pub fn table(sections: impl IntoIterator<Item = View>) -> View {
    View::Table {
        sections: sections.into_iter().collect(),
        metadata: StyleMetadata::new(ViewType::Table),
    }
}

/// Creates a semantic table header.
///
/// # Arguments
///
/// * `rows` — Table-row views rendered with the header's bold default style.
///
/// # Returns
///
/// A [`View::TableHead`] containing the provided rows.
pub fn table_head(rows: impl IntoIterator<Item = View>) -> View {
    View::TableHead {
        rows: rows.into_iter().collect(),
        metadata: StyleMetadata::new(ViewType::TableHead),
    }
}

/// Creates a semantic table body.
///
/// # Arguments
///
/// * `rows` — Table-row views rendered in source order.
///
/// # Returns
///
/// A [`View::TableBody`] containing the provided rows.
pub fn table_body(rows: impl IntoIterator<Item = View>) -> View {
    View::TableBody {
        rows: rows.into_iter().collect(),
        metadata: StyleMetadata::new(ViewType::TableBody),
    }
}

/// Creates a semantic table row.
///
/// # Arguments
///
/// * `cells` — Table-cell views rendered in column order.
///
/// # Returns
///
/// A [`View::TableRow`] containing the provided cells.
pub fn table_row(cells: impl IntoIterator<Item = View>) -> View {
    View::TableRow {
        cells: cells.into_iter().collect(),
        metadata: StyleMetadata::new(ViewType::TableRow),
    }
}

/// Creates a semantic table cell with left-aligned inline rich text.
///
/// Table cells contain Ratatui text rather than nested block views in the v1
/// semantic document API.
///
/// # Arguments
///
/// * `content` — Rich text content rendered inside the cell.
///
/// # Returns
///
/// A [`View::TableCell`] containing the provided content and default
/// [`CellAlignment::Left`] alignment.
pub fn table_cell(content: impl Into<Text<'static>>) -> View {
    View::TableCell {
        content: content.into(),
        alignment: CellAlignment::Left,
        metadata: StyleMetadata::new(ViewType::TableCell),
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

/// Creates a path-backed terminal image view.
///
/// # Arguments
///
/// * `source` — Image source to render.
///
/// # Returns
///
/// A [`View::Image`] containing the provided source.
pub fn image(source: impl Into<ImageSource>) -> View {
    View::Image {
        source: source.into(),
        alt: None,
        metadata: StyleMetadata::new(ViewType::Image),
    }
}

/// Creates a progress bar.
///
/// # Arguments
///
/// * `value` — Completion ratio rendered by the progress bar.
///
/// # Returns
///
/// A [`View::ProgressBar`] containing the provided value.
pub fn progress_bar(value: f64) -> View {
    View::ProgressBar {
        value: clamped_progress_value(value),
        label: None,
        metadata: StyleMetadata::new(ViewType::ProgressBar),
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
