//! View rendering tests.
//!
//! These tests render view trees against Ratatui's test backend and inspect the
//! resulting terminal buffer.

use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    rc::Rc,
    thread,
    time::Duration,
};

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use leptatui::{
    __private::FocusedControl,
    AlignContent, AlignItems, AnyView, AppControl, AvailableSpace, Axes, BorderType, Borders,
    BoxSizing, ButtonView, CellAlignment, Color, ContainerView, Dimension, Display, Edges,
    EditableState, FlexDirection, FlexWrap, FormView, GridLine, GridPlacement, ImageSource,
    InputView, IntoView, JustifyContent, KeyControl, LayoutGeometry, LayoutSize, Length,
    LengthAuto, LinkTarget, LinkView, ListItemView, MediaQuery, Modifier, Overflow, Position,
    RenderCtx, Result, StyleDeclarations, StyleMetadata, StyleSelector, Stylesheet, SyntaxTheme,
    TableCellView, TableRowView, TableSectionView, TextAreaView, TuiSize, TuiSpacing, TuiStyle,
    View, ViewType, VimMode, ZIndex, block, button, code_block, component, div, dynamic, form, h1,
    h2, h3, h4, h5, h6, image, input, link, list_item, markdown, ordered_list, paragraph,
    progress_bar, table, table_body, table_cell, table_head, table_row, text, text_area,
    unordered_list,
    view::{Line, Span, Text},
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::Style,
    symbols::{block as symbol_block, border as symbol_border, line as symbol_line},
};

use crate::support::{draw_view, rendered_text};

include!("support/mod.rs");

include!("boundary_fixtures.rs");
include!("boundary.rs");
include!("code_block.rs");
include!("content/mod.rs");
include!("editable_render/mod.rs");
include!("forms.rs");
include!("interaction/mod.rs");
include!("layout/mod.rs");
include!("layout_tree.rs");
include!("lists.rs");
include!("media.rs");
include!("measurement.rs");
include!("metadata.rs");
include!("progress.rs");
include!("semantic_builders.rs");
include!("links.rs");
include!("tables.rs");
