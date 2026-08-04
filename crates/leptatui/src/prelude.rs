//! Common imports for Leptatui applications.
//!
//! Application code should normally start with `use leptatui::prelude::*`.
//! This module gathers the runtime, component, view, style, context, routing,
//! async state, keyboard event, terminal key, and Leptos reactive APIs most
//! application code needs.
//!
//! The prelude exports the Leptatui `#[component]`, `view!`, and `stylesheet!`
//! macros. These macros build terminal components and [`crate::View`] trees;
//! they are not Leptos DOM macros.
//!
//! ```no_run
//! use leptatui::prelude::*;
//!
//! #[component]
//! fn Root() -> impl IntoView {
//!     view! { <Text>"Hello"</Text> }
//! }
//!
//! #[tokio::main]
//! async fn main() -> leptatui::app::Result<()> {
//!     let view = view! { <Root /> };
//!     App::new(view).run().await
//! }
//! ```
//!
//! Semantic document builders include headings, paragraphs, nested ordered or
//! unordered lists, responsive tables, syntax-highlighted code blocks, and
//! infallible in-memory or explicit path-based Markdown readers. Low-level
//! render metadata and generated-code hooks stay outside the default import.

pub use crate::{
    Action, AlignContent, AlignItems, AlignSelf, AnyView, App, AppControl, AppHandle, AppRoot,
    AvailableSpace, Axes, BlockView, BorderType, Borders, BoxSizing, ButtonView, CellAlignment,
    Children, ChildrenFn, ChildrenMut, CodeBlockView, Color, ContainerView, Dimension, Display,
    DivView, DynamicView, Edges, EditableAction, EditableState, EditableView, FlexDirection,
    FlexWrap, FormAction, FormView, Fraction, GridAutoFlow, GridLine, GridMaxTrackSize,
    GridMinTrackSize, GridPlacement, GridRepeat, GridTemplateTrack, GridTrackSize, HeadingLevel,
    HeadingView, History, ImageSource, ImageView, InputView, IntoView, IntoViews, JustifyContent,
    JustifyItems, JustifySelf, KeyControl, LayoutGeometry, LayoutSize, Length, LengthAuto,
    LinkTarget, LinkView, ListItemView, ListKind, ListView, Location, MarkdownOptions,
    MarkdownView, MediaQuery, Modifier, Navigate, NavigateOptions, Outlet, Overflow, ParagraphView,
    ParamsMap, Position, ProgressBarView, RenderCtx, Resource, RichText, RouteLinkView, Router,
    RouterProps, StyleDeclarations, StyleMetadata, StyleModule, StyleRule, StyleSelector,
    StyleValue, StyledView, Stylesheet, TableCellView, TableRowView, TableSectionKind,
    TableSectionView, TableView, TextAreaView, TextView, TextualView, ThemeValue, ThemeVariables,
    TuiSize, TuiSpacing, TuiStyle, View, ViewError, ViewResult, ViewType, ViewportSize, VimMode,
    ZIndex, block, button, code_block,
    context::{expect_context, provide_context, use_context},
    div,
    file_system::{
        FileEntry, FileKind, FileMetadata, FileOperation, FileSystem, FileSystemOptions,
        use_file_system, use_file_system_with_options,
    },
    form, h1, h2, h3, h4, h5, h6, image, input, keyed, link, list_item, markdown, markdown_file,
    markdown_file_with_options, markdown_source_with_options, markdown_with_options, ordered_list,
    paragraph, progress_bar, route_link, spawn, spawn_local, table, table_body, table_cell,
    table_head, table_row, text, text_area, theme_color, unordered_list, use_app_handle,
    use_history, use_key_event, use_location, use_navigate, use_params_map, use_query_map,
    view::{component, dynamic},
    view_error,
};

pub use leptatui_macros::{component, stylesheet, view};

pub use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub use leptos::prelude::{
    ArcRwSignal, Callback, Effect, Get, GetUntracked, IntoSignalSetter, Memo, Owner, ReadSignal,
    RenderEffect, RwSignal, Set, Signal, SignalSetter, Update, With, WithUntracked, WriteSignal,
    signal, signal_local, untrack,
};
