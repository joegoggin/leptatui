//! Basic renderable terminal views.
//!
//! This module provides a small view tree for hand-written interfaces, builder
//! functions for common and semantic view variants, Ratatui rich-text types
//! used by headings and paragraphs, nested block-oriented lists, and responsive
//! semantic tables, and bordered syntax-highlighted code blocks.

mod builders;
mod code_block;
mod component_view;
mod dynamic;
mod metadata;
mod model;
mod render;

pub(crate) use builders::component_factory;
pub use builders::{
    block, button, code_block, column, component, dynamic, form, h1, h2, h3, h4, h5, h6, image,
    input, list_item, ordered_list, paragraph, progress_bar, row, table, table_body, table_cell,
    table_head, table_row, text, text_area, unordered_list,
};
pub use code_block::SyntaxTheme;
pub use dynamic::DynamicView;
pub use metadata::{EditableState, StyleMetadata, ViewType, VimMode};
pub(crate) use model::reconcile_views;
pub use model::{
    AnyView, BlockView, ButtonAction, ButtonView, CellAlignment, CodeBlockView, ContainerView,
    EditableKind, EditableTextView, EditableView, FormAction, FormView, HeadingLevel, HeadingView,
    ImageSource, ImageView, InputAction, IntoView, IntoViews, LayoutView, ListItemView, ListKind,
    ListView, ParagraphView, ProgressBarView, StyledView, TableCellView, TableRowView,
    TableSectionKind, TableSectionView, TableView, TextView, TextualView, View,
};
pub use ratatui::text::{Line, Span, Text};
