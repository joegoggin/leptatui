//! Convenience constructors for concrete render-tree views.
//!
//! Builders retain concrete return types so type-specific configuration stays
//! available until a value is converted through [`super::IntoView`].

use ratatui::text::Text;

use crate::style::LayoutDirection;

use super::{
    code_block::{SyntaxTheme, highlighted_source_lines},
    component_view::ComponentView,
    dynamic::DynamicView,
    metadata::{EditableState, StyleMetadata, ViewType},
    model::{
        AnyView, BlockView, ButtonView, CellAlignment, CodeBlockView, EditableKind,
        EditableTextView, FormView, HeadingLevel, HeadingView, ImageSource, ImageView, IntoView,
        IntoViews, LayoutView, ListItemView, ListKind, ListView, ParagraphView, ProgressBarView,
        TableCellView, TableRowView, TableSectionKind, TableSectionView, TableView, TextView, View,
        clamped_progress_value,
    },
};

/// Creates a bordered block around one child view.
///
/// # Arguments
///
/// * `child` — View-compatible value rendered inside the block.
///
/// # Returns
///
/// A [`BlockView`] containing `child`.
pub fn block(child: impl IntoView) -> BlockView {
    BlockView {
        children: vec![child.into_view()],
        metadata: StyleMetadata::new(ViewType::Block),
    }
}

/// Creates a plain text view.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`TextView`] containing `content`.
pub fn text(content: impl Into<String>) -> TextView {
    TextView {
        content: Text::from(content.into()),
        metadata: StyleMetadata::new(ViewType::Text),
    }
}

/// Creates a semantic heading with the requested level.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
/// * `level` — Semantic heading level and selector identity.
///
/// # Returns
///
/// A [`HeadingView`] containing `content` at `level`.
fn heading(content: impl Into<Text<'static>>, level: HeadingLevel) -> HeadingView {
    HeadingView {
        content: content.into(),
        level,
        metadata: StyleMetadata::new(level.view_type()),
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
/// A first-level [`HeadingView`].
pub fn h1(content: impl Into<Text<'static>>) -> HeadingView {
    heading(content, HeadingLevel::H1)
}

/// Creates a second-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A second-level [`HeadingView`].
pub fn h2(content: impl Into<Text<'static>>) -> HeadingView {
    heading(content, HeadingLevel::H2)
}

/// Creates a third-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A third-level [`HeadingView`].
pub fn h3(content: impl Into<Text<'static>>) -> HeadingView {
    heading(content, HeadingLevel::H3)
}

/// Creates a fourth-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A fourth-level [`HeadingView`].
pub fn h4(content: impl Into<Text<'static>>) -> HeadingView {
    heading(content, HeadingLevel::H4)
}

/// Creates a fifth-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A fifth-level [`HeadingView`].
pub fn h5(content: impl Into<Text<'static>>) -> HeadingView {
    heading(content, HeadingLevel::H5)
}

/// Creates a sixth-level semantic heading.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A sixth-level [`HeadingView`].
pub fn h6(content: impl Into<Text<'static>>) -> HeadingView {
    heading(content, HeadingLevel::H6)
}

/// Creates a semantic paragraph.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`ParagraphView`] containing `content`.
pub fn paragraph(content: impl Into<Text<'static>>) -> ParagraphView {
    ParagraphView {
        content: content.into(),
        metadata: StyleMetadata::new(ViewType::Paragraph),
    }
}

/// Creates a bordered syntax-highlighted code block.
///
/// # Arguments
///
/// * `source` — Source text retained for highlighting and rendering.
///
/// # Returns
///
/// A [`CodeBlockView`] using the dark theme with line numbers disabled.
pub fn code_block(source: impl Into<String>) -> CodeBlockView {
    let source = source.into();
    CodeBlockView {
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
/// * `items` — Homogeneous collection or heterogeneous tuple of list items.
///
/// # Returns
///
/// A [`ListView`] numbered from one.
pub fn ordered_list(items: impl IntoViews) -> ListView {
    ListView {
        children: items.into_views(),
        kind: ListKind::Ordered,
        start: 1,
        metadata: StyleMetadata::new(ViewType::OrderedList),
    }
}

/// Creates a semantic unordered list.
///
/// # Arguments
///
/// * `items` — Homogeneous collection or heterogeneous tuple of list items.
///
/// # Returns
///
/// A hyphen-marked [`ListView`].
pub fn unordered_list(items: impl IntoViews) -> ListView {
    ListView {
        children: items.into_views(),
        kind: ListKind::Unordered,
        start: 1,
        metadata: StyleMetadata::new(ViewType::UnorderedList),
    }
}

/// Creates a semantic list item containing vertically stacked blocks.
///
/// # Arguments
///
/// * `children` — Homogeneous collection or heterogeneous tuple of blocks.
///
/// # Returns
///
/// A [`ListItemView`] containing the converted children.
pub fn list_item(children: impl IntoViews) -> ListItemView {
    ListItemView {
        children: children.into_views(),
        metadata: StyleMetadata::new(ViewType::ListItem),
    }
}

/// Creates a semantic table from head and body sections.
///
/// # Arguments
///
/// * `sections` — Table-head and table-body sections in source order.
///
/// # Returns
///
/// A [`TableView`] containing the converted sections.
pub fn table(sections: impl IntoViews) -> TableView {
    TableView {
        children: sections.into_views(),
        metadata: StyleMetadata::new(ViewType::Table),
    }
}

/// Creates a semantic table header.
///
/// # Arguments
///
/// * `rows` — Table rows rendered with header semantics.
///
/// # Returns
///
/// A header [`TableSectionView`].
pub fn table_head(rows: impl IntoViews) -> TableSectionView {
    TableSectionView {
        children: rows.into_views(),
        kind: TableSectionKind::Head,
        metadata: StyleMetadata::new(ViewType::TableHead),
    }
}

/// Creates a semantic table body.
///
/// # Arguments
///
/// * `rows` — Table rows rendered as body content.
///
/// # Returns
///
/// A body [`TableSectionView`].
pub fn table_body(rows: impl IntoViews) -> TableSectionView {
    TableSectionView {
        children: rows.into_views(),
        kind: TableSectionKind::Body,
        metadata: StyleMetadata::new(ViewType::TableBody),
    }
}

/// Creates a semantic table row.
///
/// # Arguments
///
/// * `cells` — Table cells in column order.
///
/// # Returns
///
/// A [`TableRowView`] containing the converted cells.
pub fn table_row(cells: impl IntoViews) -> TableRowView {
    TableRowView {
        children: cells.into_views(),
        metadata: StyleMetadata::new(ViewType::TableRow),
    }
}

/// Creates a left-aligned semantic table cell.
///
/// # Arguments
///
/// * `content` — Rich text content to render in the cell.
///
/// # Returns
///
/// A left-aligned [`TableCellView`].
pub fn table_cell(content: impl Into<Text<'static>>) -> TableCellView {
    TableCellView {
        content: content.into(),
        alignment: CellAlignment::Left,
        metadata: StyleMetadata::new(ViewType::TableCell),
    }
}

/// Creates a horizontal layout.
///
/// # Arguments
///
/// * `children` — Homogeneous collection or heterogeneous tuple of child views.
///
/// # Returns
///
/// A row-oriented [`LayoutView`].
pub fn row(children: impl IntoViews) -> LayoutView {
    LayoutView {
        children: children.into_views(),
        default_direction: LayoutDirection::Row,
        metadata: StyleMetadata::new(ViewType::Row),
    }
}

/// Creates a vertical layout.
///
/// # Arguments
///
/// * `children` — Homogeneous collection or heterogeneous tuple of child views.
///
/// # Returns
///
/// A column-oriented [`LayoutView`].
pub fn column(children: impl IntoViews) -> LayoutView {
    LayoutView {
        children: children.into_views(),
        default_direction: LayoutDirection::Column,
        metadata: StyleMetadata::new(ViewType::Column),
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

/// Creates a focusable button.
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

/// Creates a controlled single-line input.
///
/// # Arguments
///
/// * `value` — Caller-owned value displayed by the control.
///
/// # Returns
///
/// An input-configured [`EditableTextView`].
pub fn input(value: impl Into<String>) -> EditableTextView {
    editable_text(value, EditableKind::Input, ViewType::Input)
}

/// Creates a controlled multiline text area.
///
/// # Arguments
///
/// * `value` — Caller-owned value displayed by the control.
///
/// # Returns
///
/// A text-area-configured [`EditableTextView`].
pub fn text_area(value: impl Into<String>) -> EditableTextView {
    editable_text(value, EditableKind::TextArea, ViewType::TextArea)
}

/// Creates a controlled text editor with the requested geometry.
///
/// # Arguments
///
/// * `value` — Caller-owned value displayed by the control.
/// * `kind` — Single-line or multiline editing geometry.
/// * `view_type` — Stylesheet selector identity for the control.
///
/// # Returns
///
/// An [`EditableTextView`] with fresh editing state.
fn editable_text(
    value: impl Into<String>,
    kind: EditableKind,
    view_type: ViewType,
) -> EditableTextView {
    let value = value.into();
    let mut editable_state = EditableState::new();
    editable_state.set_cursor(value.len());

    EditableTextView {
        value,
        placeholder: None,
        kind,
        metadata: StyleMetadata::new(view_type),
        on_input: None,
        editable_state,
    }
}

/// Creates a path-backed terminal image view.
///
/// # Arguments
///
/// * `source` — Filesystem-backed image source.
///
/// # Returns
///
/// An [`ImageView`] with no fallback text.
pub fn image(source: impl Into<ImageSource>) -> ImageView {
    ImageView {
        source: source.into(),
        alt: None,
        metadata: StyleMetadata::new(ViewType::Image),
    }
}

/// Creates a gauge-style progress indicator.
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

/// Creates a dynamic child boundary.
///
/// # Arguments
///
/// * `child` — Closure that rebuilds the current child during traversal.
///
/// # Returns
///
/// A [`DynamicView`] retaining compatible child state between refreshes.
pub fn dynamic<V>(child: impl Fn() -> V + 'static) -> DynamicView
where
    V: IntoView,
{
    DynamicView::new(move || child().into_view())
}

/// Creates a stateful component boundary.
///
/// # Arguments
///
/// * `component` — View implementation that owns component state and context.
///
/// # Returns
///
/// An [`AnyView`] containing the component boundary.
pub fn component(component: impl View + 'static) -> AnyView {
    ComponentView::new(component).into_view()
}

/// Creates a lazy component boundary from a generated component constructor.
///
/// # Arguments
///
/// * `preserve_on_reconcile` — Whether matching generated component types may
///   retain the previous boundary.
/// * `factory` — Deferred component constructor.
///
/// # Returns
///
/// An [`AnyView`] containing the lazy component boundary.
pub(crate) fn component_factory<C>(
    preserve_on_reconcile: bool,
    factory: impl FnOnce() -> C + 'static,
) -> AnyView
where
    C: View + 'static,
{
    ComponentView::new_factory(preserve_on_reconcile, factory).into_view()
}
