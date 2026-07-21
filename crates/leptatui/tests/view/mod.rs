//! View rendering tests.
//!
//! These tests render view trees against Ratatui's test backend and inspect the
//! resulting terminal buffer.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    thread,
    time::Duration,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use leptatui::{
    __private::FocusedControl,
    AnyView, AppControl, Borders, ButtonView, CellAlignment, Color, EditableState, FormView,
    ImageSource, InputView, IntoView, KeyControl, LayoutDirection, ListItemView, MediaQuery,
    Modifier, RenderCtx, Result, StyleDeclarations, StyleMetadata, StyleSelector, Stylesheet,
    SyntaxTheme, TableCellView, TableRowView, TableSectionView, TextAreaView, TuiSize, TuiSpacing,
    TuiStyle, View, ViewType, VimMode, block, button, code_block, column, component, dynamic, form,
    h1, h2, h3, h4, h5, h6, image, input, list_item, ordered_list, paragraph, progress_bar, row,
    table, table_body, table_cell, table_head, table_row, text, text_area, unordered_list,
    view::{Line, Span, Text},
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::Style,
    symbols::{block as symbol_block, border as symbol_border, line as symbol_line},
};

use crate::support::{draw_view, rendered_text};

/// Creates a key-press event for a key code.
///
/// # Arguments
///
/// * `code` — Key code to place in the generated event.
///
/// # Returns
///
/// An [`Event`] containing the key press.
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Returns flattened focus states for all buttons in a view tree.
///
/// # Arguments
///
/// * `view` — View tree to inspect.
///
/// # Returns
///
/// A [`Vec<bool>`] containing focus state for each button.
fn button_focuses(view: &dyn View) -> Vec<bool> {
    if let Some(button) = view.as_any().downcast_ref::<ButtonView>() {
        return vec![button.metadata().is_focused()];
    }

    view.children()
        .iter()
        .flat_map(|child| button_focuses(child.as_view()))
        .collect()
}

/// Returns flattened focus states for all focusable controls in a view tree.
///
/// # Arguments
///
/// * `view` — View tree to inspect.
///
/// # Returns
///
/// A [`Vec<bool>`] containing focus state for each focusable control.
fn control_focuses(view: &dyn View) -> Vec<bool> {
    if let Some(button) = view.as_any().downcast_ref::<ButtonView>() {
        return vec![button.metadata().is_focused()];
    }
    if let Some(editor) = view.as_any().downcast_ref::<InputView>() {
        return vec![editor.metadata().is_focused()];
    }
    if let Some(editor) = view.as_any().downcast_ref::<TextAreaView>() {
        return vec![editor.metadata().is_focused()];
    }

    view.children()
        .iter()
        .flat_map(|child| control_focuses(child.as_view()))
        .collect()
}

/// Creates an editable input test view.
///
/// # Arguments
///
/// * `value` — Caller-owned value to display in the input.
///
/// # Returns
///
/// A [`View`] containing an input with fresh editable state.
fn editable_input(value: impl Into<String>) -> InputView {
    input(value)
}

/// Creates an editable text-area test view.
///
/// # Arguments
///
/// * `value` — Caller-owned value to display in the text area.
///
/// # Returns
///
/// A [`View`] containing a text area with fresh editable state.
fn editable_text_area(value: impl Into<String>) -> TextAreaView {
    text_area(value)
}

/// Creates non-default editable state for reconciliation tests.
///
/// # Returns
///
/// An [`EditableState`] value containing cursor, scroll, mode, selection, yank,
/// undo, and redo state.
fn editable_state_fixture() -> EditableState {
    let mut state = EditableState::new();
    state.set_cursor(6);
    state.set_horizontal_scroll(2);
    state.set_vertical_scroll(3);
    state.set_mode(VimMode::Visual);
    state.set_selection_anchor(Some(2));
    state.set_yank_buffer("copied");
    state.push_undo("before");
    state.push_redo("after");
    state
}

/// Returns editable state stored by an editable test view.
///
/// Panics if `view` is not an editable control.
///
/// # Arguments
///
/// * `view` — Editable view to inspect.
///
/// # Returns
///
/// An [`EditableState`] reference retained by the view.
fn editable_state(view: &dyn View) -> &EditableState {
    if let Some(view) = view.as_any().downcast_ref::<InputView>() {
        return view.editable_state();
    }
    view.as_any()
        .downcast_ref::<TextAreaView>()
        .expect("expected editable view")
        .editable_state()
}

/// Returns mutable editable state stored by an editable test view.
///
/// Panics if `view` is not an editable control.
///
/// # Arguments
///
/// * `view` — Editable view to mutate.
///
/// # Returns
///
/// An [`EditableState`] reference retained by the view.
fn editable_state_mut(view: &mut dyn View) -> &mut EditableState {
    if view.as_any().is::<InputView>() {
        return view
            .as_any_mut()
            .downcast_mut::<InputView>()
            .expect("expected input view")
            .editable_state_mut();
    }
    view.as_any_mut()
        .downcast_mut::<TextAreaView>()
        .expect("expected text-area view")
        .editable_state_mut()
}

/// Returns an unmodified key event for a test key code.
///
/// # Arguments
///
/// * `code` — Key code to wrap in a [`KeyEvent`].
///
/// # Returns
///
/// A [`KeyEvent`] value without modifiers.
fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Returns a control-modified character key event for tests.
///
/// # Arguments
///
/// * `character` — Character to wrap in a control-modified [`KeyEvent`].
///
/// # Returns
///
/// A [`KeyEvent`] value with the control modifier set.
fn ctrl_key_event(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

/// Returns a control-modified enter key event for tests.
///
/// # Returns
///
/// A [`KeyEvent`] value with enter and the control modifier set.
fn ctrl_enter_key_event() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL)
}

/// Creates a focused input view that records emitted values.
///
/// # Arguments
///
/// * `value` — Initial controlled input value.
/// * `emitted` — Shared vector that receives callback values.
///
/// # Returns
///
/// A focused insert-mode [`View`] configured as an input.
fn emitting_input(value: impl Into<String>, emitted: &Rc<RefCell<Vec<String>>>) -> InputView {
    let emitted_for_input = Rc::clone(emitted);
    let mut view = input(value).with_focus(true).on_input(move |next| {
        emitted_for_input.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut view).set_mode(VimMode::Insert);
    view
}

/// Creates a focused text-area view that records emitted values.
///
/// # Arguments
///
/// * `value` — Initial controlled text-area value.
/// * `emitted` — Shared vector that receives callback values.
///
/// # Returns
///
/// A focused insert-mode [`View`] configured as a text area.
fn emitting_text_area(
    value: impl Into<String>,
    emitted: &Rc<RefCell<Vec<String>>>,
) -> TextAreaView {
    let emitted_for_text_area = Rc::clone(emitted);
    let mut view = text_area(value).with_focus(true).on_input(move |next| {
        emitted_for_text_area.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut view).set_mode(VimMode::Insert);
    view
}

/// Returns a reconciled input view with a new controlled value.
///
/// # Arguments
///
/// * `previous` — Previous input view whose retained metadata should be reused.
/// * `value` — Next controlled input value.
/// * `emitted` — Shared vector that receives callback values.
///
/// # Returns
///
/// A [`View`] containing the reconciled input.
fn reconcile_input_value(
    previous: &InputView,
    value: impl Into<String>,
    emitted: &Rc<RefCell<Vec<String>>>,
) -> InputView {
    let mut next = emitting_input(value, emitted);
    leptatui::__private::__reconcile_view(&mut next, previous);
    next
}

/// Returns a reconciled text-area view with a new controlled value.
///
/// # Arguments
///
/// * `previous` — Previous text-area view whose retained metadata should be
///   reused.
/// * `value` — Next controlled text-area value.
/// * `emitted` — Shared vector that receives callback values.
///
/// # Returns
///
/// A [`View`] containing the reconciled text area.
fn reconcile_text_area_value(
    previous: &TextAreaView,
    value: impl Into<String>,
    emitted: &Rc<RefCell<Vec<String>>>,
) -> TextAreaView {
    let mut next = emitting_text_area(value, emitted);
    leptatui::__private::__reconcile_view(&mut next, previous);
    next
}

/// Creates a controlled form test view backed by shared caller-owned state.
///
/// # Arguments
///
/// * `name` — Shared controlled input value.
/// * `notes` — Shared controlled text-area value.
/// * `submits` — Shared form submit count.
/// * `cancels` — Shared form cancel count.
///
/// # Returns
///
/// A [`View`] containing a form with an input, text area, and submit button.
fn controlled_form_view(
    name: &Rc<RefCell<String>>,
    notes: &Rc<RefCell<String>>,
    submits: &Rc<Cell<usize>>,
    cancels: &Rc<Cell<usize>>,
) -> FormView {
    let name_value = name.borrow().clone();
    let notes_value = notes.borrow().clone();
    let name_for_input = Rc::clone(name);
    let notes_for_text_area = Rc::clone(notes);
    let submits_for_form = Rc::clone(submits);
    let cancels_for_form = Rc::clone(cancels);

    form((
        input(name_value).placeholder("Name").on_input(move |next| {
            *name_for_input.borrow_mut() = next;
            AppControl::Continue
        }),
        text_area(notes_value)
            .placeholder("Notes")
            .on_input(move |next| {
                *notes_for_text_area.borrow_mut() = next;
                AppControl::Continue
            }),
        button("Submit"),
    ))
    .on_submit(move || {
        submits_for_form.set(submits_for_form.get() + 1);
        AppControl::Continue
    })
    .on_cancel(move || {
        cancels_for_form.set(cancels_for_form.get() + 1);
        AppControl::Continue
    })
}

/// Returns a reconciled controlled form from the latest shared state.
///
/// # Arguments
///
/// * `previous` — Previous controlled form view.
/// * `name` — Shared controlled input value.
/// * `notes` — Shared controlled text-area value.
/// * `submits` — Shared form submit count.
/// * `cancels` — Shared form cancel count.
///
/// # Returns
///
/// A [`View`] containing the next controlled form with retained editable state.
fn reconcile_controlled_form(
    previous: &FormView,
    name: &Rc<RefCell<String>>,
    notes: &Rc<RefCell<String>>,
    submits: &Rc<Cell<usize>>,
    cancels: &Rc<Cell<usize>>,
) -> FormView {
    let mut next = controlled_form_view(name, notes, submits, cancels);
    leptatui::__private::__reconcile_view(&mut next, previous);
    next
}

/// Returns a child from a controlled form by index.
///
/// # Arguments
///
/// * `view` — Form view to inspect.
/// * `index` — Child index to return.
///
/// # Returns
///
/// A [`View`] reference for the requested form child.
fn form_child(view: &FormView, index: usize) -> &dyn View {
    view.children()[index].as_view()
}

/// Returns the controlled value from an input view.
///
/// # Arguments
///
/// * `view` — Input view to inspect.
///
/// # Returns
///
/// A string slice containing the input's controlled value.
fn input_value(view: &dyn View) -> &str {
    let editor = view
        .as_any()
        .downcast_ref::<InputView>()
        .expect("expected input view");
    editor.value()
}

/// Returns the controlled value from a text-area view.
///
/// # Arguments
///
/// * `view` — Text-area view to inspect.
///
/// # Returns
///
/// A string slice containing the text area's controlled value.
fn text_area_value(view: &dyn View) -> &str {
    let editor = view
        .as_any()
        .downcast_ref::<TextAreaView>()
        .expect("expected text-area view");
    editor.value()
}

/// Returns the scroll offset from a layout view.
///
/// # Arguments
///
/// * `view` — Row, column, or form view to inspect.
///
/// # Returns
///
/// A [`u16`] containing the current vertical scroll offset.
fn scroll_offset(view: &dyn View) -> u16 {
    view.style_metadata()
        .expect("expected styleable layout view")
        .scroll_offset()
}

/// Returns the position of a rendered terminal cell symbol.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
/// * `symbol` — Cell symbol to locate.
/// * `width` — Terminal width used to convert buffer index to coordinates.
///
/// # Returns
///
/// A [`tuple`](prim@tuple) containing the `x` and `y` coordinates.
///
/// # Panics
///
/// Panics if no rendered cell has the requested symbol.
fn symbol_position(terminal: &Terminal<TestBackend>, symbol: &str, width: u16) -> (u16, u16) {
    symbol_position_opt(terminal, symbol, width)
        .unwrap_or_else(|| panic!("rendered `{symbol}` cell"))
}

/// Returns the optional position of a rendered terminal cell symbol.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
/// * `symbol` — Cell symbol to locate.
/// * `width` — Terminal width used to convert buffer index to coordinates.
///
/// # Returns
///
/// An [`Option`] containing the `x` and `y` coordinates when the symbol exists.
fn symbol_position_opt(
    terminal: &Terminal<TestBackend>,
    symbol: &str,
    width: u16,
) -> Option<(u16, u16)> {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .enumerate()
        .find_map(|(index, cell)| {
            (cell.symbol() == symbol).then(|| {
                let index = index as u16;
                (index % width, index / width)
            })
        })
}

/// Returns the symbol rendered at a terminal coordinate.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
/// * `x` — Horizontal cell coordinate.
/// * `y` — Vertical cell coordinate.
/// * `width` — Terminal width used to convert coordinates to a buffer index.
///
/// # Returns
///
/// A string slice containing the rendered cell symbol.
fn cell_symbol(terminal: &Terminal<TestBackend>, x: u16, y: u16, width: u16) -> &str {
    let index = usize::from(y) * usize::from(width) + usize::from(x);
    terminal.backend().buffer().content()[index].symbol()
}

/// Returns foreground and background colors at a terminal coordinate.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
/// * `x` — Horizontal cell coordinate.
/// * `y` — Vertical cell coordinate.
/// * `width` — Terminal width used to convert coordinates to a buffer index.
///
/// # Returns
///
/// A [`tuple`](prim@tuple) containing foreground and background colors.
fn cell_colors(terminal: &Terminal<TestBackend>, x: u16, y: u16, width: u16) -> (Color, Color) {
    let index = usize::from(y) * usize::from(width) + usize::from(x);
    let cell = &terminal.backend().buffer().content()[index];
    (cell.fg, cell.bg)
}

/// Returns text modifiers at a terminal coordinate.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
/// * `x` — Horizontal cell coordinate.
/// * `y` — Vertical cell coordinate.
/// * `width` — Terminal width used to convert coordinates to a buffer index.
///
/// # Returns
///
/// A [`Modifier`] value containing the rendered cell modifiers.
fn cell_modifiers(terminal: &Terminal<TestBackend>, x: u16, y: u16, width: u16) -> Modifier {
    let index = usize::from(y) * usize::from(width) + usize::from(x);
    terminal.backend().buffer().content()[index].modifier
}

include!("boundary.rs");
include!("code_block.rs");
include!("content.rs");
include!("editable_render.rs");
include!("forms.rs");
include!("interaction.rs");
include!("layout.rs");
include!("lists.rs");
include!("media.rs");
include!("metadata.rs");
include!("progress.rs");
include!("semantic_builders.rs");
include!("tables.rs");
