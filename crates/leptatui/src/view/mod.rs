//! Basic renderable terminal views.
//!
//! Concrete views are organized by domain while this module keeps the public
//! constructor and type surface flat for application code.
//!
//! # Modules
//!
//! - `boundary` — Component and dynamic view boundaries.
//! - `containers` — Layout, list, and table views.
//! - `content` — Text, paragraph, heading, and code views.
//! - `controls` — Interactive form and input views.
//! - `core` — Shared view contracts, conversion, metadata, and rendering.
//! - `error` — Default interactive screen for fallible component errors.
//! - `media` — Terminal image views.
//! - `reconciliation` — Retained view-tree state reconciliation.

pub(crate) mod boundary;
pub(crate) mod containers;
pub(crate) mod content;
pub(crate) mod controls;
pub(crate) mod core;
pub(crate) mod error;
mod link;
pub(crate) mod media;
mod reconciliation;

pub(crate) use boundary::component::{ComponentView, WeakComponentView, component_factory};
pub use boundary::{
    component::component,
    dynamic::{DynamicView, dynamic, keyed},
};
pub use containers::{
    block::{BlockView, block},
    layout::{DivView, div},
    lists::{
        list::{ListKind, ListView, ordered_list, unordered_list},
        list_item::{ListItemView, list_item},
    },
    tables::{
        table::{TableView, table},
        table_cell::{CellAlignment, TableCellView, table_cell},
        table_row::{TableRowView, table_row},
        table_section::{TableSectionKind, TableSectionView, table_body, table_head},
    },
};
pub use content::{
    code_block::{CodeBlockView, SyntaxTheme, code_block},
    heading::{HeadingLevel, HeadingView, h1, h2, h3, h4, h5, h6},
    paragraph::{ParagraphView, paragraph},
    text::{TextView, text},
};
pub use controls::{
    button::{ButtonAction, ButtonView, button},
    editable::{
        input::{InputView, input},
        model::EditableAction,
        state::{EditableState, VimMode},
        text_area::{TextAreaView, text_area},
    },
    form::{FormAction, FormView, form},
    link::{LinkView, link},
    progress_bar::{ProgressBarView, progress_bar},
    route_link::{RouteLinkView, route_link},
};
pub use core::{
    any_view::AnyView,
    capabilities::{ContainerView, EditableView, StyledView, TextualView},
    contract::View,
    conversion::{IntoView, IntoViews},
    measurement::AvailableSpace,
    metadata::{LayoutGeometry, StyleMetadata, ViewType},
};
pub(crate) use link::{InlineLink, LinkedSpan};
pub use link::{LinkTarget, RichText};
pub use media::image::{ImageSource, ImageView, image};
pub use ratatui::text::{Line, Span, Text};
pub(crate) use reconciliation::reconcile_views;
