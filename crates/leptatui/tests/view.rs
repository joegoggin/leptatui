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
    __private::FocusedControl, AppControl, AppRoot, Borders, Color, Component, EditableState,
    ImageSource, KeyControl, LayoutDirection, MediaQuery, Modifier, RenderCtx, Result,
    StyleMetadata, StyleSelector, Stylesheet, TuiStyle, View, ViewType, VimMode, block, button,
    column, component, dynamic, form, image, input, progress_bar, row, text, text_area,
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    symbols::{block as symbol_block, border as symbol_border, line as symbol_line},
};

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
fn button_focuses(view: &View) -> Vec<bool> {
    match view {
        View::Button { metadata, .. } => vec![metadata.is_focused()],
        View::Block { child, .. } => button_focuses(child),
        View::Row { children, .. }
        | View::Column { children, .. }
        | View::Form { children, .. } => children.iter().flat_map(button_focuses).collect(),
        View::Text { .. }
        | View::Input { .. }
        | View::TextArea { .. }
        | View::Image { .. }
        | View::ProgressBar { .. }
        | View::Dynamic(_)
        | View::Component(_) => Vec::new(),
    }
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
fn control_focuses(view: &View) -> Vec<bool> {
    match view {
        View::Button { metadata, .. }
        | View::Input { metadata, .. }
        | View::TextArea { metadata, .. } => vec![metadata.is_focused()],
        View::Block { child, .. } => control_focuses(child),
        View::Row { children, .. }
        | View::Column { children, .. }
        | View::Form { children, .. } => children.iter().flat_map(control_focuses).collect(),
        View::Text { .. }
        | View::Image { .. }
        | View::ProgressBar { .. }
        | View::Dynamic(_)
        | View::Component(_) => Vec::new(),
    }
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
fn editable_input(value: impl Into<String>) -> View {
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
fn editable_text_area(value: impl Into<String>) -> View {
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
fn editable_state(view: &View) -> &EditableState {
    match view {
        View::Input { editable_state, .. } | View::TextArea { editable_state, .. } => {
            editable_state
        }
        other => panic!("expected editable view, got {other:?}"),
    }
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
fn editable_state_mut(view: &mut View) -> &mut EditableState {
    match view {
        View::Input { editable_state, .. } | View::TextArea { editable_state, .. } => {
            editable_state
        }
        other => panic!("expected editable view, got {other:?}"),
    }
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
fn emitting_input(value: impl Into<String>, emitted: &Rc<RefCell<Vec<String>>>) -> View {
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
fn emitting_text_area(value: impl Into<String>, emitted: &Rc<RefCell<Vec<String>>>) -> View {
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
    previous: &View,
    value: impl Into<String>,
    emitted: &Rc<RefCell<Vec<String>>>,
) -> View {
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
    previous: &View,
    value: impl Into<String>,
    emitted: &Rc<RefCell<Vec<String>>>,
) -> View {
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
) -> View {
    let name_value = name.borrow().clone();
    let notes_value = notes.borrow().clone();
    let name_for_input = Rc::clone(name);
    let notes_for_text_area = Rc::clone(notes);
    let submits_for_form = Rc::clone(submits);
    let cancels_for_form = Rc::clone(cancels);

    form([
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
    ])
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
    previous: &View,
    name: &Rc<RefCell<String>>,
    notes: &Rc<RefCell<String>>,
    submits: &Rc<Cell<usize>>,
    cancels: &Rc<Cell<usize>>,
) -> View {
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
fn form_child(view: &View, index: usize) -> &View {
    match view {
        View::Form { children, .. } => &children[index],
        other => panic!("expected form view, got {other:?}"),
    }
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
fn input_value(view: &View) -> &str {
    match view {
        View::Input { value, .. } => value,
        other => panic!("expected input view, got {other:?}"),
    }
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
fn text_area_value(view: &View) -> &str {
    match view {
        View::TextArea { value, .. } => value,
        other => panic!("expected text-area view, got {other:?}"),
    }
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
fn scroll_offset(view: &View) -> u16 {
    match view {
        View::Row { metadata, .. }
        | View::Column { metadata, .. }
        | View::Form { metadata, .. } => metadata.scroll_offset(),
        other => panic!("expected layout view, got {other:?}"),
    }
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

/// Draws a view into a test terminal.
///
/// # Arguments
///
/// * `terminal` — Test terminal receiving the rendered view.
/// * `view` — View tree to render.
///
/// # Returns
///
/// An empty [`Result`] on successful terminal and view rendering.
///
/// # Errors
///
/// Returns [`leptatui::Error`] if terminal drawing or view rendering fails.
fn draw_view(terminal: &mut Terminal<TestBackend>, view: &View) -> Result<()> {
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;

    render_result
}

/// Returns the rendered buffer symbols as one contiguous string.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
///
/// # Returns
///
/// A [`String`] containing every cell symbol in buffer order.
fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

/// Verifies a block view renders its child text.
///
/// # Example Under Test
///
/// ```text
/// block(text("Hello"))
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rendered buffer contains `Hello`.
#[test]
fn renders_block_and_text_views() -> Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = block(text("Hello")).render(&mut ctx);
    })?;
    render_result?;

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Hello"));

    Ok(())
}

/// Verifies text views render with stylesheet-resolved colors.
///
/// # Example Under Test
///
/// ```text
/// text("Hi").with_classes("accent")
/// Stylesheet::new().rule(StyleSelector::class("accent"), yellow on blue)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rendered `H` cell exists.
/// - The rendered `H` cell has a yellow foreground.
/// - The rendered `H` cell has a blue background.
#[test]
fn renders_text_with_resolved_stylesheet_style() -> Result<()> {
    let backend = TestBackend::new(12, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = text("Hi").with_classes("accent");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("accent"),
        TuiStyle::new()
            .foreground(Color::Yellow)
            .background(Color::Blue),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "H")
        .expect("rendered H cell");

    assert_eq!(cell.fg, Color::Yellow);
    assert_eq!(cell.bg, Color::Blue);

    Ok(())
}

/// Verifies text wraps to the available render width.
///
/// # Example Under Test
///
/// ```text
/// text("Hello World")
/// terminal width = 6
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - `Hello` starts on the first row.
/// - `World` starts on the second row.
#[test]
fn text_wraps_to_available_render_width() -> Result<()> {
    let backend = TestBackend::new(6, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = text("Hello World");
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "H", 6), (0, 0));
    assert_eq!(symbol_position(&terminal, "W", 6), (0, 1));

    Ok(())
}

/// Verifies input views render their controlled value.
///
/// # Example Under Test
///
/// ```text
/// input("Ada")
/// width = 8, height = 3
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The input renders a default border.
/// - The inner cells contain `A`, `d`, and `a`.
#[test]
fn renders_input_value() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = input("Ada");

    draw_view(&mut terminal, &view)?;

    assert_eq!(
        cell_symbol(&terminal, 0, 0, 8),
        symbol_border::PLAIN.top_left
    );
    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "A");
    assert_eq!(cell_symbol(&terminal, 2, 1, 8), "d");
    assert_eq!(cell_symbol(&terminal, 3, 1, 8), "a");

    Ok(())
}

/// Verifies input default borders can be disabled through styles.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_inline_style(TuiStyle::new().borders(Borders::NONE))
/// width = 8
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The value starts in the first cell when borders are disabled.
#[test]
fn input_borders_none_disables_default_border() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = input("Ada").with_inline_style(TuiStyle::new().borders(Borders::NONE));

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "A");
    assert_eq!(cell_symbol(&terminal, 3, 0, 8), " ");

    Ok(())
}

/// Verifies empty input views render placeholder text.
///
/// # Example Under Test
///
/// ```text
/// input("").placeholder("Name")
/// width = 8, height = 3
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The inner cells contain the first and last placeholder characters.
#[test]
fn renders_input_placeholder_when_value_is_empty() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = input("").placeholder("Name");

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "N");
    assert_eq!(cell_symbol(&terminal, 4, 1, 8), "e");

    Ok(())
}

/// Verifies focused input views receive focus stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// :focus { fg: Black, bg: Yellow }
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component render call succeeds.
/// - The focused input cell uses the stylesheet foreground color.
/// - The focused input cell uses the stylesheet background color.
#[test]
fn renders_focused_input_with_focus_stylesheet_rule() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = input("Ada").with_focus(true);
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::focus(),
        TuiStyle::new()
            .foreground(Color::Black)
            .background(Color::Yellow),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, bg) = cell_colors(&terminal, 1, 1, 8);
    assert_eq!(fg, Color::Black);
    assert_eq!(bg, Color::Yellow);

    Ok(())
}

/// Verifies focused inputs place the terminal cursor at the retained cursor.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// cursor = end
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The focused input sets the terminal cursor after the rendered value.
#[test]
fn focused_input_sets_terminal_cursor_position() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = input("Ada").with_focus(true);

    draw_view(&mut terminal, &view)?;

    terminal.backend_mut().assert_cursor_position((4, 1));

    Ok(())
}

/// Verifies component-backed roots expose focused editable control mode.
#[test]
fn app_root_reports_focused_editable_control_mode() -> Result<()> {
    let normal_input = input("Ada").with_focus(true);
    assert_eq!(
        AppRoot::__focused_control(&normal_input),
        Some(FocusedControl::Input {
            insert_mode: false,
            visual_mode: false,
        })
    );

    let mut insert_input = input("Ada").with_focus(true);
    editable_state_mut(&mut insert_input).set_mode(VimMode::Insert);
    assert_eq!(
        AppRoot::__focused_control(&insert_input),
        Some(FocusedControl::Input {
            insert_mode: true,
            visual_mode: false,
        })
    );

    insert_input.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(
        AppRoot::__focused_control(&insert_input),
        Some(FocusedControl::Input {
            insert_mode: false,
            visual_mode: false,
        })
    );

    let mut visual_input = input("Ada").with_focus(true);
    editable_state_mut(&mut visual_input).set_mode(VimMode::Visual);
    editable_state_mut(&mut visual_input).set_selection_anchor(Some(0));
    assert_eq!(
        AppRoot::__focused_control(&visual_input),
        Some(FocusedControl::Input {
            insert_mode: false,
            visual_mode: true,
        })
    );

    let normal_text_area = text_area("Ada").with_focus(true);
    assert_eq!(
        AppRoot::__focused_control(&normal_text_area),
        Some(FocusedControl::TextArea {
            insert_mode: false,
            visual_mode: false,
        })
    );

    let mut insert_text_area = text_area("Ada").with_focus(true);
    editable_state_mut(&mut insert_text_area).set_mode(VimMode::Insert);
    assert_eq!(
        AppRoot::__focused_control(&insert_text_area),
        Some(FocusedControl::TextArea {
            insert_mode: true,
            visual_mode: false,
        })
    );

    insert_text_area.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(
        AppRoot::__focused_control(&insert_text_area),
        Some(FocusedControl::TextArea {
            insert_mode: false,
            visual_mode: false,
        })
    );

    let mut visual_text_area = text_area("Ada").with_focus(true);
    editable_state_mut(&mut visual_text_area).set_mode(VimMode::VisualLine);
    editable_state_mut(&mut visual_text_area).set_selection_anchor(Some(0));
    assert_eq!(
        AppRoot::__focused_control(&visual_text_area),
        Some(FocusedControl::TextArea {
            insert_mode: false,
            visual_mode: true,
        })
    );

    assert_eq!(
        AppRoot::__focused_control(&button("Save").with_focus(true)),
        Some(FocusedControl::Button)
    );
    assert_eq!(AppRoot::__focused_control(&input("Ada")), None);

    Ok(())
}

/// Verifies input rendering clips content around the retained cursor.
///
/// # Example Under Test
///
/// ```text
/// input("abcdef").with_focus(true)
/// width = 4, height = 3
/// cursor = end, then cursor = 0
/// ```
///
/// # Assertions
///
/// - The first render succeeds and shows the tail of the value.
/// - Moving the cursor to the start succeeds.
/// - The second render succeeds and shows the head of the value.
#[test]
fn input_rendering_clips_and_scrolls_around_cursor() -> Result<()> {
    let backend = TestBackend::new(4, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("abcdef").with_focus(true);

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 4), "e");
    assert_eq!(cell_symbol(&terminal, 2, 1, 4), "f");

    editable_state_mut(&mut view).set_cursor(0);
    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 4), "a");
    assert_eq!(cell_symbol(&terminal, 2, 1, 4), "b");

    Ok(())
}

/// Verifies visual-mode input selections render selected cells in reverse video.
#[test]
fn input_visual_selection_renders_reversed_cells() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("abcd").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Visual);
    editable_state_mut(&mut view).set_selection_anchor(Some(1));
    editable_state_mut(&mut view).set_cursor(2);

    draw_view(&mut terminal, &view)?;

    assert!(!cell_modifiers(&terminal, 1, 1, 8).contains(Modifier::REVERSED));
    assert!(cell_modifiers(&terminal, 2, 1, 8).contains(Modifier::REVERSED));
    assert!(cell_modifiers(&terminal, 3, 1, 8).contains(Modifier::REVERSED));
    assert!(!cell_modifiers(&terminal, 4, 1, 8).contains(Modifier::REVERSED));

    Ok(())
}

/// Verifies a pending insert-mode `j` renders as a reversed preview character.
#[test]
fn input_pending_insert_j_renders_reversed_preview() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("Ada").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Insert);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 4, 1, 8), "j");
    assert!(cell_modifiers(&terminal, 4, 1, 8).contains(Modifier::REVERSED));
    terminal.backend_mut().assert_cursor_position((4, 1));

    Ok(())
}

/// Verifies an expired pending insert-mode `j` renders without preview styling.
#[test]
fn input_pending_insert_j_preview_expires_to_insert_cursor() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("Ada").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Insert);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    thread::sleep(Duration::from_millis(1100));
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 4, 1, 8), "j");
    assert!(!cell_modifiers(&terminal, 4, 1, 8).contains(Modifier::REVERSED));
    terminal.backend_mut().assert_cursor_position((5, 1));
    assert_eq!(
        AppRoot::__focused_control(&view),
        Some(FocusedControl::Input {
            insert_mode: true,
            visual_mode: false,
        })
    );

    Ok(())
}

/// Verifies text-area views render multiline controlled values.
///
/// # Example Under Test
///
/// ```text
/// text_area("One\nTwo")
/// width = 8, height = 4
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The text area renders a default border.
/// - The first line starts on the first inner row.
/// - The second line starts on the second inner row.
#[test]
fn renders_text_area_multiline_value() -> Result<()> {
    let backend = TestBackend::new(8, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("One\nTwo");

    draw_view(&mut terminal, &view)?;

    assert_eq!(
        cell_symbol(&terminal, 0, 0, 8),
        symbol_border::PLAIN.top_left
    );
    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "O");
    assert_eq!(cell_symbol(&terminal, 1, 2, 8), "T");

    Ok(())
}

/// Verifies text-area default borders can be disabled through styles.
///
/// # Example Under Test
///
/// ```text
/// text_area("One\nTwo").with_inline_style(TuiStyle::new().borders(Borders::NONE))
/// width = 8, height = 2
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - Lines start in the first column when borders are disabled.
#[test]
fn text_area_borders_none_disables_default_border() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("One\nTwo").with_inline_style(TuiStyle::new().borders(Borders::NONE));

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "O");
    assert_eq!(cell_symbol(&terminal, 0, 1, 8), "T");

    Ok(())
}

/// Verifies empty text areas render placeholder text.
///
/// # Example Under Test
///
/// ```text
/// text_area("").placeholder("Notes")
/// width = 8, height = 3
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The inner cells contain the first and last placeholder characters.
#[test]
fn renders_text_area_placeholder_when_value_is_empty() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("").placeholder("Notes");

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "N");
    assert_eq!(cell_symbol(&terminal, 5, 1, 8), "s");

    Ok(())
}

/// Verifies focused text areas receive focus stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada").with_focus(true)
/// :focus { fg: Black, bg: Yellow }
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component render call succeeds.
/// - The focused text-area cell uses the stylesheet foreground color.
/// - The focused text-area cell uses the stylesheet background color.
#[test]
fn renders_focused_text_area_with_focus_stylesheet_rule() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("Ada").with_focus(true);
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::focus(),
        TuiStyle::new()
            .foreground(Color::Black)
            .background(Color::Yellow),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, bg) = cell_colors(&terminal, 1, 1, 8);
    assert_eq!(fg, Color::Black);
    assert_eq!(bg, Color::Yellow);

    Ok(())
}

/// Verifies focused text areas place the terminal cursor at the retained cursor.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo").with_focus(true)
/// cursor = end
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The focused text area sets the terminal cursor on the final row.
#[test]
fn focused_text_area_sets_terminal_cursor_position() -> Result<()> {
    let backend = TestBackend::new(8, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("one\ntwo").with_focus(true);

    draw_view(&mut terminal, &view)?;

    terminal.backend_mut().assert_cursor_position((4, 2));

    Ok(())
}

/// Verifies text-area rendering scrolls vertically around the retained cursor.
///
/// # Example Under Test
///
/// ```text
/// text_area("aaa\nbbb\nccc").with_focus(true)
/// height = 4
/// cursor = end, then cursor = 0
/// ```
///
/// # Assertions
///
/// - The first render succeeds and shows the tail of the multiline value.
/// - Moving the cursor to the start succeeds.
/// - The second render succeeds and shows the head of the multiline value.
#[test]
fn text_area_rendering_scrolls_vertically_around_cursor() -> Result<()> {
    let backend = TestBackend::new(8, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = text_area("aaa\nbbb\nccc").with_focus(true);

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "b");
    assert_eq!(cell_symbol(&terminal, 1, 2, 8), "c");

    editable_state_mut(&mut view).set_cursor(0);
    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "a");
    assert_eq!(cell_symbol(&terminal, 1, 2, 8), "b");

    Ok(())
}

/// Verifies visual-line text-area selections render selected lines in reverse video.
#[test]
fn text_area_visual_line_selection_renders_reversed_cells() -> Result<()> {
    let backend = TestBackend::new(10, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut view = text_area("one\ntwo\nthree").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::VisualLine);
    editable_state_mut(&mut view).set_selection_anchor(Some(4));
    editable_state_mut(&mut view).set_cursor(8);

    draw_view(&mut terminal, &view)?;

    assert!(!cell_modifiers(&terminal, 1, 1, 10).contains(Modifier::REVERSED));
    assert!(cell_modifiers(&terminal, 1, 2, 10).contains(Modifier::REVERSED));
    assert!(cell_modifiers(&terminal, 1, 3, 10).contains(Modifier::REVERSED));

    Ok(())
}

/// Verifies a wrapped pending insert-mode `j` renders where the preview wraps.
#[test]
fn text_area_pending_insert_j_renders_reversed_wrapped_preview() -> Result<()> {
    let backend = TestBackend::new(5, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = text_area("Ada").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Insert);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 1, 2, 5), "j");
    assert!(cell_modifiers(&terminal, 1, 2, 5).contains(Modifier::REVERSED));
    terminal.backend_mut().assert_cursor_position((1, 2));

    Ok(())
}

/// Verifies columns reserve multiline text-area render height.
///
/// # Example Under Test
///
/// ```text
/// column([text_area("Hello World"), text("End")])
/// width = 6
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The following text view renders after the wrapped text-area rows.
#[test]
fn column_reserves_height_for_wrapped_text_area() -> Result<()> {
    let backend = TestBackend::new(6, 7);
    let mut terminal = Terminal::new(backend)?;
    let view = column(vec![text_area("Hello World"), text("End")]);

    draw_view(&mut terminal, &view)?;

    assert_eq!(symbol_position(&terminal, "E", 6), (0, 6));

    Ok(())
}

/// Verifies columns reserve wrapped text render height.
///
/// # Example Under Test
///
/// ```text
/// column([text("Hello World"), text("End")])
/// width = 6
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - Wrapped text occupies the first two rows.
/// - The following text view renders on the third row.
#[test]
fn column_reserves_height_for_wrapped_text() -> Result<()> {
    let backend = TestBackend::new(6, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = column(vec![text("Hello World"), text("End")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "W", 6).1, 1);
    assert_eq!(symbol_position(&terminal, "E", 6).1, 2);

    Ok(())
}

/// Verifies form views render children and participate in focus traversal.
///
/// # Example Under Test
///
/// ```text
/// form([text("Title"), input("Ada"), button("Save")])
/// Tab, Tab
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The title renders before the input value.
/// - The form reports two focusable descendant controls.
/// - Tab moves focus from the input to the button.
#[test]
fn renders_form_children_and_moves_focus_through_descendants() -> Result<()> {
    let backend = TestBackend::new(12, 7);
    let mut terminal = Terminal::new(backend)?;
    let mut view = form([text("Title"), input("Ada"), button("Save")]);

    draw_view(&mut terminal, &view)?;

    let title_position = symbol_position(&terminal, "T", 12);
    let input_position = symbol_position(&terminal, "A", 12);
    assert_eq!(title_position, (0, 0));
    assert_eq!(input_position.0, 1);
    assert!(input_position.1 > title_position.1);
    assert_eq!(view.__focusable_count(), 2);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![true, false]);
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![false, true]);

    Ok(())
}

/// Verifies focusing an editable control starts it in normal mode.
///
/// # Example Under Test
///
/// ```text
/// form([input("Ada").with_focus(true), button("Save")])
/// Tab, Tab
/// ```
///
/// # Assertions
///
/// - Moving focus away does not discard retained editable state.
/// - Moving focus back to the input switches it to normal mode.
/// - Cursor and yank buffer state are preserved.
#[test]
fn focusing_editable_control_enters_normal_mode_without_resetting_state() -> Result<()> {
    let mut input_view = input("Ada").with_focus(true);
    editable_state_mut(&mut input_view).set_mode(VimMode::Insert);
    editable_state_mut(&mut input_view).set_cursor(1);
    editable_state_mut(&mut input_view).set_yank_buffer("copy");
    let mut view = form([input_view, button("Save")]);

    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Insert);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![false, true]);
    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Insert);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![true, false]);
    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Normal);
    assert_eq!(editable_state(form_child(&view, 0)).cursor(), 1);
    assert_eq!(editable_state(form_child(&view, 0)).yank_buffer(), "copy");

    Ok(())
}

/// Verifies form type stylesheet rules apply through rendered descendants.
///
/// # Example Under Test
///
/// ```text
/// Form { fg: Green }
/// form([text("Hi")])
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The rendered text cell inherits the form foreground color.
#[test]
fn form_type_styles_apply_to_descendants() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = form([text("Hi")]);
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::Form),
        TuiStyle::new().foreground(Color::Green),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, _) = cell_colors(&terminal, 0, 0, 8);
    assert_eq!(fg, Color::Green);

    Ok(())
}

/// Verifies controlled form edits update caller state and reconcile into views.
///
/// # Example Under Test
///
/// ```text
/// form([input(name), text_area(notes), button("Submit")])
/// Tab, A, Char('!'), reconcile, Tab, A, Enter, reconcile
/// :focus { fg: Black, bg: Yellow }
/// ```
///
/// # Assertions
///
/// - The form reports input, text area, and button as focusable controls.
/// - Input edits update caller-owned state without mutating the stale view.
/// - Reconciliation displays the latest caller-owned input value and retains
///   focus and cursor state.
/// - Text-area edits follow the same controlled update and reconciliation path.
/// - The focused text area receives focus stylesheet colors after reconciliation.
#[test]
fn controlled_form_reconciles_values_focus_and_rendering() -> Result<()> {
    let name = Rc::new(RefCell::new(String::from("Ada")));
    let notes = Rc::new(RefCell::new(String::from("Notes")));
    let submits = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let mut view = controlled_form_view(&name, &notes, &submits, &cancels);

    assert_eq!(view.__focusable_count(), 3);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![true, false, false]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('A')))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('!')))?,
        KeyControl::Handled
    );
    assert_eq!(name.borrow().as_str(), "Ada!");
    assert_eq!(input_value(form_child(&view, 0)), "Ada");

    view = reconcile_controlled_form(&view, &name, &notes, &submits, &cancels);

    assert_eq!(input_value(form_child(&view, 0)), "Ada!");
    assert_eq!(editable_state(form_child(&view, 0)).cursor(), 4);
    assert_eq!(control_focuses(&view), vec![true, false, false]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![false, true, false]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('A')))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(notes.borrow().as_str(), "Notes\n");
    assert_eq!(text_area_value(form_child(&view, 1)), "Notes");

    view = reconcile_controlled_form(&view, &name, &notes, &submits, &cancels);

    assert_eq!(text_area_value(form_child(&view, 1)), "Notes\n");
    assert_eq!(editable_state(form_child(&view, 1)).cursor(), 6);
    assert_eq!(control_focuses(&view), vec![false, true, false]);

    let backend = TestBackend::new(20, 6);
    let mut terminal = Terminal::new(backend)?;
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::focus(),
        TuiStyle::new()
            .foreground(Color::Black)
            .background(Color::Yellow),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.fg == Color::Black && cell.bg == Color::Yellow)
    );

    Ok(())
}

/// Verifies form-owned submit and cancel keys route around editable controls.
///
/// # Example Under Test
///
/// ```text
/// form([input(name), text_area(notes), button("Submit")])
/// Input focus: Enter, i, Esc, Esc
/// TextArea focus: A, Enter, reconcile, Ctrl+Enter
/// ```
///
/// # Assertions
///
/// - Enter submits a form when the input is focused.
/// - Esc leaves input insert mode without canceling the form.
/// - Esc in normal mode invokes the form cancel callback.
/// - Plain Enter inserts a newline when the text area is in insert mode.
/// - Ctrl+Enter submits a form when the text area is focused.
#[test]
fn controlled_form_routes_submit_and_cancel_keys() -> Result<()> {
    let name = Rc::new(RefCell::new(String::from("Ada")));
    let notes = Rc::new(RefCell::new(String::from("Notes")));
    let submits = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let mut input_view = controlled_form_view(&name, &notes, &submits, &cancels);

    input_view.handle_key_event(key_event(KeyCode::Tab))?;
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(submits.get(), 1);
    assert_eq!(cancels.get(), 0);

    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Char('i')))?,
        KeyControl::Handled
    );
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(
        editable_state(form_child(&input_view, 0)).mode(),
        VimMode::Normal
    );
    assert_eq!(cancels.get(), 0);

    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 1);

    let name = Rc::new(RefCell::new(String::from("Ada")));
    let notes = Rc::new(RefCell::new(String::from("Notes")));
    let submits = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let mut text_area_view = controlled_form_view(&name, &notes, &submits, &cancels);

    text_area_view.handle_key_event(key_event(KeyCode::Tab))?;
    text_area_view.handle_key_event(key_event(KeyCode::Tab))?;
    assert_eq!(
        text_area_view.handle_key_event(key_event(KeyCode::Char('A')))?,
        KeyControl::Handled
    );
    assert_eq!(
        text_area_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(notes.borrow().as_str(), "Notes\n");
    assert_eq!(submits.get(), 0);

    text_area_view = reconcile_controlled_form(&text_area_view, &name, &notes, &submits, &cancels);

    assert_eq!(
        text_area_view.handle_key_event(ctrl_enter_key_event())?,
        KeyControl::Handled
    );
    assert_eq!(notes.borrow().as_str(), "Notes\n");
    assert_eq!(submits.get(), 1);

    Ok(())
}

/// Verifies controlled form Vim edits survive reconciled redraws.
///
/// # Example Under Test
///
/// ```text
/// Input focus: 0, l, x, reconcile, u, reconcile, Ctrl+r
/// TextArea focus: gg, j, dd, reconcile, u
/// ```
///
/// # Assertions
///
/// - Normal-mode input deletion updates caller-owned state.
/// - Reconciliation retains input undo and redo history.
/// - Normal-mode text-area line deletion updates caller-owned state.
/// - Reconciliation retains the linewise yank buffer for text-area undo.
#[test]
fn controlled_form_preserves_vim_state_across_reconciliation() -> Result<()> {
    let name = Rc::new(RefCell::new(String::from("abc")));
    let notes = Rc::new(RefCell::new(String::from("notes")));
    let submits = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let mut input_view = controlled_form_view(&name, &notes, &submits, &cancels);

    input_view.handle_key_event(key_event(KeyCode::Tab))?;
    input_view.handle_key_event(key_event(KeyCode::Char('0')))?;
    input_view.handle_key_event(key_event(KeyCode::Char('l')))?;
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Char('x')))?,
        KeyControl::Handled
    );
    assert_eq!(name.borrow().as_str(), "ac");

    input_view = reconcile_controlled_form(&input_view, &name, &notes, &submits, &cancels);
    assert_eq!(input_value(form_child(&input_view, 0)), "ac");
    assert_eq!(
        editable_state(form_child(&input_view, 0)).undo_stack(),
        &[String::from("abc")]
    );

    input_view.handle_key_event(key_event(KeyCode::Char('u')))?;
    assert_eq!(name.borrow().as_str(), "abc");

    input_view = reconcile_controlled_form(&input_view, &name, &notes, &submits, &cancels);
    assert_eq!(
        input_view.handle_key_event(ctrl_key_event('r'))?,
        KeyControl::Handled
    );
    assert_eq!(name.borrow().as_str(), "ac");

    let name = Rc::new(RefCell::new(String::from("Ada")));
    let notes = Rc::new(RefCell::new(String::from("one\ntwo\nthree")));
    let submits = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let mut text_area_view = controlled_form_view(&name, &notes, &submits, &cancels);

    text_area_view.handle_key_event(key_event(KeyCode::Tab))?;
    text_area_view.handle_key_event(key_event(KeyCode::Tab))?;
    text_area_view.handle_key_event(key_event(KeyCode::Char('g')))?;
    text_area_view.handle_key_event(key_event(KeyCode::Char('g')))?;
    text_area_view.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(
        text_area_view.handle_key_event(key_event(KeyCode::Char('d')))?,
        KeyControl::Handled
    );
    assert_eq!(
        text_area_view.handle_key_event(key_event(KeyCode::Char('d')))?,
        KeyControl::Handled
    );
    assert_eq!(notes.borrow().as_str(), "one\nthree");

    text_area_view = reconcile_controlled_form(&text_area_view, &name, &notes, &submits, &cancels);
    assert_eq!(
        text_area_value(form_child(&text_area_view, 1)),
        "one\nthree"
    );
    assert_eq!(
        editable_state(form_child(&text_area_view, 1)).yank_buffer(),
        "two"
    );

    text_area_view.handle_key_event(key_event(KeyCode::Char('u')))?;
    assert_eq!(notes.borrow().as_str(), "one\ntwo\nthree");

    Ok(())
}

/// Verifies fitting columns do not render a scrollbar.
///
/// # Example Under Test
///
/// ```text
/// column([text("12345678")])
/// terminal size = 8x1
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rightmost cell remains the final text character.
#[test]
fn fitting_column_does_not_render_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = column([text("12345678")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), "8");

    Ok(())
}

/// Verifies overflowing columns render a right-side scrollbar.
///
/// # Example Under Test
///
/// ```text
/// column([text("One"), text("Two"), text("Three")])
/// terminal size = 8x2
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The first scrollbar cell renders as the scroll thumb.
/// - The second scrollbar cell renders as the scrollbar track.
#[test]
fn overflowing_column_renders_right_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = column(vec![text("One"), text("Two"), text("Three")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), symbol_block::FULL);
    assert_eq!(
        cell_symbol(&terminal, 7, 1, 8),
        symbol_line::DOUBLE_VERTICAL
    );

    Ok(())
}

/// Verifies dynamic overflowing columns keep scroll metadata between refreshes.
///
/// # Example Under Test
///
/// ```text
/// dynamic(|| column([text("One"), text("Two"), text("Three")]))
/// terminal size = 8x2
/// ```
///
/// # Assertions
///
/// - Initial rendering measures overflow.
/// - The Down key is handled by the refreshed dynamic child.
/// - Rendering after the key shows the scrolled second row.
#[test]
fn dynamic_overflowing_column_scrolls_after_render() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = dynamic(|| column(vec![text("One"), text("Two"), text("Three")]));

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "O");

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Down))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "T");
    assert_eq!(cell_symbol(&terminal, 1, 0, 8), "w");

    Ok(())
}

/// Verifies overflowing columns reserve width for the scrollbar.
///
/// # Example Under Test
///
/// ```text
/// column([text("123456"), text("more"), text("tail")])
/// terminal size = 6x2
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - Text wraps before the scrollbar column.
/// - The scrollbar thumb occupies the rightmost column.
#[test]
fn overflowing_column_reserves_width_for_scrollbar() -> Result<()> {
    let backend = TestBackend::new(6, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = column(vec![text("123456"), text("more"), text("tail")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 4, 0, 6), "5");
    assert_eq!(cell_symbol(&terminal, 5, 0, 6), symbol_block::FULL);
    assert_eq!(cell_symbol(&terminal, 0, 1, 6), "6");

    Ok(())
}

/// Verifies overflowing columns update the scrollbar thumb after scrolling.
///
/// # Example Under Test
///
/// ```text
/// column([text("One"), text("Two"), text("Three")])
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial terminal draw succeeds.
/// - PageDown is handled by the view.
/// - The second terminal draw succeeds.
/// - The scrollbar thumb moves from the top cell to the bottom cell.
#[test]
fn overflowing_column_updates_scrollbar_position() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column(vec![text("One"), text("Two"), text("Three")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(
        cell_symbol(&terminal, 7, 0, 8),
        symbol_line::DOUBLE_VERTICAL
    );
    assert_eq!(cell_symbol(&terminal, 7, 1, 8), symbol_block::FULL);

    Ok(())
}

/// Verifies overflowing column scrollbars reach the bottom at max scroll.
///
/// # Example Under Test
///
/// ```text
/// column(Line 0..Line 9)
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial terminal draw succeeds.
/// - PageDown is handled by the view.
/// - The second terminal draw succeeds.
/// - The scrollbar thumb reaches the bottom row.
#[test]
fn overflowing_column_scrollbar_reaches_bottom_at_max_scroll() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = column(children.collect::<Vec<_>>());
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 4, 8), symbol_block::FULL);

    Ok(())
}

/// Verifies Vim `G` scrolls an overflowing column to the bottom.
///
/// # Example Under Test
///
/// ```text
/// column(Line 0..Line 9)
/// G
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds.
/// - The initial scroll offset is zero.
/// - `G` is handled by the view.
/// - The scroll offset moves to the bottom.
#[test]
fn overflowing_column_scrolls_to_bottom_with_vim_g() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = column(children.collect::<Vec<_>>());

    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 0);
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 5);

    Ok(())
}

/// Verifies Vim `gg` scrolls an overflowing column to the top.
///
/// # Example Under Test
///
/// ```text
/// column(Line 0..Line 9)
/// G, g, g
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds.
/// - `G` scrolls to the bottom.
/// - The first `g` keeps the pending top-scroll prefix.
/// - The second `g` scrolls to the top.
#[test]
fn overflowing_column_scrolls_to_top_with_vim_gg() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = column(children.collect::<Vec<_>>());

    draw_view(&mut terminal, &view)?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 5);

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 5);
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}

/// Verifies the Vim `gg` prefix resets after an unrelated key.
///
/// # Example Under Test
///
/// ```text
/// G, g, Down, g, g
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds.
/// - `G` scrolls to the bottom.
/// - `g`, `Down`, `g` leaves the scroll offset at the bottom.
/// - A fresh `g` completes the prefix and scrolls to the top.
#[test]
fn vim_scroll_to_top_prefix_resets_on_unrelated_key() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = column(children.collect::<Vec<_>>());

    draw_view(&mut terminal, &view)?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 5);

    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 5);

    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}

/// Verifies parent backgrounds fill the bottom row after scrolling down.
///
/// # Example Under Test
///
/// ```text
/// column([text("Top"), button("Launch"), text("Tail")]).surface
/// Down
/// ```
///
/// # Assertions
///
/// - The initial styled render succeeds.
/// - The down key is handled by the view.
/// - The second styled render succeeds.
/// - Empty and occupied cells on the bottom row keep the parent background.
#[test]
fn overflowing_column_keeps_parent_background_on_bottom_row_after_scrolling_down() -> Result<()> {
    let backend = TestBackend::new(12, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view =
        column(vec![text("Top"), button("Launch"), text("Tail")]).with_classes("surface");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("surface"),
        TuiStyle::new().background(Color::Blue),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(cell_colors(&terminal, 0, 1, 12).1, Color::Blue);
    assert_eq!(cell_colors(&terminal, 5, 1, 12).1, Color::Blue);

    Ok(())
}

/// Verifies parent backgrounds fill the top row after scrolling up.
///
/// # Example Under Test
///
/// ```text
/// column([text("Top"), button("Launch"), text("Tail")]).surface
/// PageDown, Up
/// ```
///
/// # Assertions
///
/// - The initial styled render succeeds.
/// - PageDown and Up are handled by the view.
/// - The second styled render succeeds.
/// - Empty and occupied cells on the top row keep the parent background.
#[test]
fn overflowing_column_keeps_parent_background_on_top_row_after_scrolling_up() -> Result<()> {
    let backend = TestBackend::new(12, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view =
        column(vec![text("Top"), button("Launch"), text("Tail")]).with_classes("surface");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("surface"),
        TuiStyle::new().background(Color::Blue),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(cell_colors(&terminal, 0, 0, 12).1, Color::Blue);
    assert_eq!(cell_colors(&terminal, 5, 0, 12).1, Color::Blue);

    Ok(())
}

/// Verifies row minimum height uses split child widths for wrapped text.
///
/// # Example Under Test
///
/// ```text
/// row([text("Hello World"), text("Side")])
/// terminal width = 12
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The row minimum height accounts for wrapping inside the split child area.
#[test]
fn row_min_height_uses_split_child_widths_for_wrapped_text() -> Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = row(vec![text("Hello World"), text("Side")]);
    let mut min_height = 0;

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = view.__min_height(&mut ctx);
    })?;

    assert_eq!(min_height, 2);

    Ok(())
}

/// Verifies text-area minimum height counts trailing newline rows.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\n")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The text area minimum height includes the trailing blank line and border.
#[test]
fn text_area_min_height_counts_trailing_newline() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("Ada\n");
    let mut min_height = 0;

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = view.__min_height(&mut ctx);
    })?;

    assert_eq!(min_height, 4);

    Ok(())
}

/// Verifies component boundaries backed by [`View`] report wrapped view height.
///
/// # Example Under Test
///
/// ```text
/// component(column([text("One"), text("Two"), text("Three")])).__min_height(ctx)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component boundary reports the wrapped column's three-row height.
///
/// # Why
///
/// Parent layouts use component minimum heights to decide whether children need
/// fixed height or overflow scrolling.
#[test]
fn component_view_min_height_uses_wrapped_view_height() -> Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = component(column([text("One"), text("Two"), text("Three")]));
    let mut min_height = 0;

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = view.__min_height(&mut ctx);
    })?;

    assert_eq!(min_height, 3);

    Ok(())
}

/// Verifies overflowing columns scroll rows produced by wrapped text.
///
/// # Example Under Test
///
/// ```text
/// column([text("Hello World"), text("Bottom")])
/// terminal size = 7x2
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial render succeeds and hides the later child.
/// - PageDown is handled by the view.
/// - The second render succeeds.
/// - The wrapped text row and later child become visible after scrolling.
#[test]
fn overflowing_column_scrolls_wrapped_text_rows() -> Result<()> {
    let backend = TestBackend::new(7, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column(vec![text("Hello World"), text("Bottom")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "B", 7).is_none());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "W", 7).1, 0);
    assert_eq!(symbol_position(&terminal, "B", 7).1, 1);

    Ok(())
}

/// Verifies render contexts apply media rules using the root viewport.
///
/// # Example Under Test
///
/// ```text
/// @media (max-width: 12) { .accent { fg: Yellow } }
/// terminal width = 12
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rendered text resolves the media-rule foreground color.
#[test]
fn render_context_applies_media_rules_from_root_viewport() -> Result<()> {
    let backend = TestBackend::new(12, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = text("Hi").with_classes("accent");
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("accent"),
        TuiStyle::new().foreground(Color::Yellow),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "H")
        .expect("rendered H cell");

    assert_eq!(cell.fg, Color::Yellow);

    Ok(())
}

/// Verifies media direction gives stacked bordered buttons enough height.
///
/// # Example Under Test
///
/// ```text
/// row([button("A"), button("B")]).stack
/// @media (max-width: 12) { .stack { direction: Column } }
/// ```
///
/// # Assertions
///
/// - The styled render succeeds.
/// - The first bordered button renders near the top.
/// - The second bordered button renders lower after column stacking.
#[test]
fn media_direction_gives_stacked_bordered_buttons_minimum_height() -> Result<()> {
    let backend = TestBackend::new(12, 6);
    let mut terminal = Terminal::new(backend)?;
    let view = row(vec![button("A"), button("B")]).with_classes("stack");
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("stack"),
        TuiStyle::new().direction(LayoutDirection::Column),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "A", 12).1, 1);
    assert_eq!(symbol_position(&terminal, "B", 12).1, 4);

    Ok(())
}

/// Verifies columns reserve height for nested media-stacked bordered buttons.
///
/// # Example Under Test
///
/// ```text
/// column([text("Top"), row(buttons).stack, text("End")])
/// @media (max-width: 12) { .stack { direction: Column } }
/// ```
///
/// # Assertions
///
/// - The styled render succeeds.
/// - The fourth nested button renders on the expected lower row.
#[test]
fn column_reserves_height_for_nested_stacked_bordered_buttons() -> Result<()> {
    let backend = TestBackend::new(12, 14);
    let mut terminal = Terminal::new(backend)?;
    let view = column(vec![
        text("Top"),
        row(vec![button("A"), button("B"), button("C"), button("D")]).with_classes("stack"),
        text("End"),
    ]);
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("stack"),
        TuiStyle::new().direction(LayoutDirection::Column),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "D", 12).1, 11);

    Ok(())
}

/// Verifies overflowing columns can scroll to later children by default.
///
/// # Example Under Test
///
/// ```text
/// column([text rows..., row([button("Launch"), button("Quit")]).focus-actions])
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial styled render succeeds and hides the later button.
/// - PageDown is handled by the view.
/// - The second styled render succeeds and shows the later button.
#[test]
fn overflowing_column_scrolls_to_later_children_by_default() -> Result<()> {
    let backend = TestBackend::new(12, 6);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column(vec![
        text("One"),
        text("Two"),
        text("Three"),
        text("Four"),
        row(vec![button("Launch"), button("Quit")]).with_classes("focus-actions"),
    ]);
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("focus-actions"),
        TuiStyle::new().direction(LayoutDirection::Column),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "Q", 12).is_none());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "Q", 12).1, 4);

    Ok(())
}

/// Verifies page scrolling handles stacked buttons without nested scrolling.
///
/// # Example Under Test
///
/// ```text
/// column([block(text("Top")), row(buttons).stack, text("End")])
/// PageDown, PageDown
/// ```
///
/// # Assertions
///
/// - The first render shows the top block and hides the last button.
/// - The first PageDown scrolls the parent page and reveals middle buttons.
/// - The second PageDown keeps the top block hidden and reveals the last button.
///
/// # Why
///
/// Parent overflow should manage page scrolling when nested stacked content is
/// taller than the viewport.
#[test]
fn overflowing_page_scrolls_stacked_buttons_without_nested_scroll() -> Result<()> {
    let backend = TestBackend::new(12, 6);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column(vec![
        block(text("Top")),
        row(vec![button("A"), button("B"), button("C"), button("D")]).with_classes("stack"),
        text("End"),
    ]);
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("stack"),
        TuiStyle::new().direction(LayoutDirection::Column),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "T", 12).is_some());
    assert!(symbol_position_opt(&terminal, "D", 12).is_none());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "T", 12).is_none());
    assert!(symbol_position_opt(&terminal, "C", 12).is_some());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "T", 12).is_none());
    assert!(symbol_position_opt(&terminal, "D", 12).is_some());

    Ok(())
}

/// Verifies row layout stays horizontal without a direction override.
///
/// # Example Under Test
///
/// ```text
/// row([text("A"), text("B")])
/// terminal width = 4
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The child text views render on the same row in separate columns.
#[test]
fn row_layout_stays_horizontal_without_direction_override() -> Result<()> {
    let backend = TestBackend::new(4, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = row(vec![text("A"), text("B")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "A", 4), (0, 0));
    assert_eq!(symbol_position(&terminal, "B", 4), (2, 0));

    Ok(())
}

/// Verifies view builders store default selector metadata.
///
/// # Example Under Test
///
/// ```text
/// block(text("child"))
/// ```
///
/// # Assertions
///
/// - Block metadata is available.
/// - The view type is `Block`.
/// - The metadata has no id.
/// - The metadata has no classes.
/// - The metadata has no inline style.
/// - The metadata is not focused.
#[test]
fn view_builders_store_default_selector_metadata() {
    let block_view = block(text("child"));
    let metadata = block_view.style_metadata().expect("block metadata");

    assert_eq!(metadata.view_type(), ViewType::Block);
    assert_eq!(metadata.id(), None);
    assert!(metadata.classes().is_empty());
    assert_eq!(metadata.inline_style(), None);
    assert!(!metadata.is_focused());
}

/// Verifies view metadata setters store selector fields.
///
/// # Example Under Test
///
/// ```text
/// button("Save")
///     .with_id("save")
///     .with_classes("primary toolbar")
///     .with_inline_style(yellow)
///     .with_focus(true)
/// ```
///
/// # Assertions
///
/// - Button metadata is available.
/// - The view type is `Button`.
/// - The metadata id is `save`.
/// - The metadata classes are `primary` and `toolbar`.
/// - The metadata inline style is yellow.
/// - The metadata is focused.
#[test]
fn view_metadata_setters_store_selector_fields() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view = button("Save")
        .with_id("save")
        .with_classes("primary toolbar")
        .with_inline_style(style)
        .with_focus(true);
    let metadata = view.style_metadata().expect("button metadata");

    assert_eq!(metadata.view_type(), ViewType::Button);
    assert_eq!(metadata.id(), Some("save"));
    assert_eq!(
        metadata.classes(),
        &[String::from("primary"), String::from("toolbar")]
    );
    assert_eq!(metadata.inline_style(), Some(style));
    assert!(metadata.is_focused());
}

/// Verifies image builders store source, fallback text, and selector metadata.
///
/// # Example Under Test
///
/// ```text
/// image("assets/logo.png")
///     .alt("Project logo")
///     .with_id("logo")
///     .with_classes("media primary")
/// ```
///
/// # Assertions
///
/// - The image source is path-backed.
/// - Fallback text is retained.
/// - The metadata view type is `Image`.
/// - Standard selector metadata is retained.
#[test]
fn image_builder_stores_source_alt_and_selector_metadata() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view = image("assets/logo.png")
        .alt("Project logo")
        .with_id("logo")
        .with_classes("media primary")
        .with_inline_style(style);

    match view {
        View::Image {
            source,
            alt,
            metadata,
        } => {
            assert_eq!(source, ImageSource::Path("assets/logo.png".into()));
            assert_eq!(alt.as_deref(), Some("Project logo"));
            assert_eq!(metadata.view_type(), ViewType::Image);
            assert_eq!(metadata.id(), Some("logo"));
            assert_eq!(
                metadata.classes(),
                &[String::from("media"), String::from("primary")]
            );
            assert_eq!(metadata.inline_style(), Some(style));
        }
        other => panic!("expected image view, got {other:?}"),
    }
}

/// Verifies image fallback rendering prefers caller-provided alt text.
///
/// # Example Under Test
///
/// ```text
/// image("missing.png").alt("Project logo")
/// TestBackend
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The fallback text is rendered into the test backend.
/// - No escape protocol bytes are written into the text buffer.
///
/// # Why
///
/// Test backends must remain deterministic even when terminal image protocols
/// are unavailable.
#[test]
fn image_fallback_renders_alt_text_on_test_backend() -> Result<()> {
    let backend = TestBackend::new(24, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = image("missing.png").alt("Project logo");

    draw_view(&mut terminal, &view)?;

    let rendered = rendered_text(&terminal);
    assert!(rendered.contains("Project logo"));
    assert!(!rendered.contains('\u{1b}'));

    Ok(())
}

/// Verifies image fallback rendering has deterministic text without alt text.
///
/// # Example Under Test
///
/// ```text
/// image("missing.png")
/// TestBackend
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The rendered fallback text matches the runtime deterministic support
///   message.
#[test]
fn image_fallback_without_alt_uses_support_message() -> Result<()> {
    let backend = TestBackend::new(40, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = image("missing.png");

    draw_view(&mut terminal, &view)?;

    let expected = "terminal image support is unavailable";
    assert!(rendered_text(&terminal).contains(expected));

    Ok(())
}

/// Verifies image type styles apply to fallback text.
///
/// # Example Under Test
///
/// ```text
/// Image { fg: Green }
/// image("missing.png").alt("Logo")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The fallback text resolves styles through `ViewType::Image`.
#[test]
fn image_type_styles_apply_to_fallback_text() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = image("missing.png").alt("Logo");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::Image),
        TuiStyle::new().foreground(Color::Green),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, _) = cell_colors(&terminal, 0, 0, 8);
    assert_eq!(fg, Color::Green);

    Ok(())
}

/// Verifies image fallback text inherits parent text styles.
///
/// # Example Under Test
///
/// ```text
/// Form { fg: Green }
/// form([image("missing.png").alt("Logo")])
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The image fallback cell inherits foreground color from the form.
#[test]
fn image_fallback_text_inherits_parent_text_style() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = form([image("missing.png").alt("Logo")]);
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::Form),
        TuiStyle::new().foreground(Color::Green),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, _) = cell_colors(&terminal, 0, 0, 8);
    assert_eq!(fg, Color::Green);

    Ok(())
}

/// Verifies progress bar builders store value, label, and selector metadata.
///
/// # Example Under Test
///
/// ```text
/// progress_bar(0.5)
///     .label("Loading")
///     .with_id("upload")
///     .with_classes("meter primary")
///     .with_inline_style(yellow)
/// ```
///
/// # Assertions
///
/// - The progress value is retained.
/// - The optional label is retained.
/// - The metadata view type is `ProgressBar`.
/// - Standard selector metadata is retained.
/// - Out-of-range builder values are clamped.
#[test]
fn progress_bar_builder_stores_value_label_and_selector_metadata() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view = progress_bar(0.5)
        .label("Loading")
        .with_id("upload")
        .with_classes("meter primary")
        .with_inline_style(style);

    match view {
        View::ProgressBar {
            value,
            label,
            metadata,
        } => {
            assert_eq!(value, 0.5);
            assert_eq!(label.as_deref(), Some("Loading"));
            assert_eq!(metadata.view_type(), ViewType::ProgressBar);
            assert_eq!(metadata.id(), Some("upload"));
            assert_eq!(
                metadata.classes(),
                &[String::from("meter"), String::from("primary")]
            );
            assert_eq!(metadata.inline_style(), Some(style));
        }
        other => panic!("expected progress bar view, got {other:?}"),
    }

    match progress_bar(1.5) {
        View::ProgressBar { value, .. } => assert_eq!(value, 1.0),
        other => panic!("expected progress bar view, got {other:?}"),
    }

    match progress_bar(f64::NAN) {
        View::ProgressBar { value, .. } => assert_eq!(value, 0.0),
        other => panic!("expected progress bar view, got {other:?}"),
    }
}

/// Verifies empty, partial, and full progress values render as gauges.
///
/// # Example Under Test
///
/// ```text
/// progress_bar(0.0)
/// progress_bar(0.5)
/// progress_bar(1.0)
/// ```
///
/// # Assertions
///
/// - Empty progress renders without filled cells.
/// - Partial progress renders filled cells.
/// - Full progress fills both edges around Ratatui's centered label.
#[test]
fn progress_bar_renders_empty_partial_and_full_values() -> Result<()> {
    let mut empty_terminal = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut empty_terminal, &progress_bar(0.0))?;
    assert!(!rendered_text(&empty_terminal).contains(symbol_block::FULL));

    let mut partial_terminal = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut partial_terminal, &progress_bar(0.5))?;
    assert_eq!(cell_symbol(&partial_terminal, 0, 0, 10), symbol_block::FULL);
    assert_ne!(cell_symbol(&partial_terminal, 9, 0, 10), symbol_block::FULL);

    let mut full_terminal = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut full_terminal, &progress_bar(1.0))?;
    assert_eq!(cell_symbol(&full_terminal, 0, 0, 10), symbol_block::FULL);
    assert_eq!(cell_symbol(&full_terminal, 9, 0, 10), symbol_block::FULL);

    Ok(())
}

/// Verifies progress values are clamped before rendering.
///
/// # Example Under Test
///
/// ```text
/// progress_bar(-0.5)
/// progress_bar(1.5)
/// progress_bar(f64::NAN)
/// ```
///
/// # Assertions
///
/// - Underflow renders the same as `0.0`.
/// - Overflow renders the same as `1.0`.
/// - Non-finite progress renders the same as `0.0`.
#[test]
fn progress_bar_clamps_values_before_rendering() -> Result<()> {
    let mut underflow = Terminal::new(TestBackend::new(10, 1))?;
    let mut empty = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut underflow, &progress_bar(-0.5))?;
    draw_view(&mut empty, &progress_bar(0.0))?;
    assert_eq!(rendered_text(&underflow), rendered_text(&empty));

    let mut overflow = Terminal::new(TestBackend::new(10, 1))?;
    let mut full = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut overflow, &progress_bar(1.5))?;
    draw_view(&mut full, &progress_bar(1.0))?;
    assert_eq!(rendered_text(&overflow), rendered_text(&full));

    let mut non_finite = Terminal::new(TestBackend::new(10, 1))?;
    let mut empty_again = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut non_finite, &progress_bar(f64::NAN))?;
    draw_view(&mut empty_again, &progress_bar(0.0))?;
    assert_eq!(rendered_text(&non_finite), rendered_text(&empty_again));

    Ok(())
}

/// Verifies progress bar labels render over the gauge.
///
/// # Example Under Test
///
/// ```text
/// progress_bar(0.5).label("Uploading")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The caller-provided label appears in the rendered buffer.
#[test]
fn progress_bar_renders_optional_label() -> Result<()> {
    let backend = TestBackend::new(20, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = progress_bar(0.5).label("Uploading");

    draw_view(&mut terminal, &view)?;

    assert!(rendered_text(&terminal).contains("Uploading"));

    Ok(())
}

/// Verifies progress bar type styles apply to the gauge.
///
/// # Example Under Test
///
/// ```text
/// ProgressBar { fg: Green, bg: Blue }
/// progress_bar(1.0).label("Done")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The gauge resolves styles through `ViewType::ProgressBar`.
#[test]
fn progress_bar_type_styles_apply_to_gauge() -> Result<()> {
    let backend = TestBackend::new(12, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = progress_bar(1.0).label("Done");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::ProgressBar),
        TuiStyle::new()
            .foreground(Color::Green)
            .background(Color::Blue),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, bg) = cell_colors(&terminal, 0, 0, 12);
    assert_eq!(fg, Color::Green);
    assert_eq!(bg, Color::Blue);

    Ok(())
}

/// Verifies progress bars do not participate in built-in focus traversal.
///
/// # Example Under Test
///
/// ```text
/// column([progress_bar(0.5), button("Save")])
/// Tab
/// ```
///
/// # Assertions
///
/// - Only the button is counted as focusable.
/// - Tab focuses the button and skips the progress bar.
#[test]
fn progress_bar_is_not_focusable() -> Result<()> {
    let mut view = column([progress_bar(0.5), button("Save")]);

    assert_eq!(view.__focusable_count(), 1);
    assert_eq!(control_focuses(&view), vec![false]);
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![true]);

    Ok(())
}

/// Verifies tab navigation moves between static buttons.
///
/// # Example Under Test
///
/// ```text
/// column([button("One"), text("Gap"), button("Two")])
/// Tab, Tab, BackTab
/// ```
///
/// # Assertions
///
/// - The first tab event succeeds and focuses the first button.
/// - The second tab event succeeds and focuses the second button.
/// - The back-tab event succeeds and returns focus to the first button.
///
/// # Why
///
/// Non-focusable text views should be skipped during keyboard focus movement.
#[test]
fn tab_focus_moves_between_static_buttons() -> Result<()> {
    let mut view = column([button("One"), text("Gap"), button("Two")]);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(button_focuses(&view), vec![true, false]);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(button_focuses(&view), vec![false, true]);

    view.handle_event(key(KeyCode::BackTab))?;
    assert_eq!(button_focuses(&view), vec![true, false]);

    Ok(())
}

/// Verifies tab navigation includes editable controls.
///
/// # Example Under Test
///
/// ```text
/// column([button("Save"), text("Gap"), Input, TextArea, button("Submit")])
/// Tab x4, BackTab
/// ```
///
/// # Assertions
///
/// - The view reports four focusable controls.
/// - Each tab event succeeds and moves focus in render order.
/// - The back-tab event succeeds and moves focus back one control.
/// - Non-editable text is skipped.
#[test]
fn tab_focus_moves_across_buttons_and_editable_controls() -> Result<()> {
    let mut view = column([
        button("Save"),
        text("Gap"),
        editable_input("Ada"),
        editable_text_area("Notes"),
        button("Submit"),
    ]);

    assert_eq!(view.__focusable_count(), 4);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![true, false, false, false]);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![false, true, false, false]);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![false, false, true, false]);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![false, false, false, true]);

    view.handle_event(key(KeyCode::BackTab))?;
    assert_eq!(control_focuses(&view), vec![false, false, true, false]);

    Ok(())
}

/// Verifies tab focus scrolls an overflowing column to the focused button.
///
/// # Example Under Test
///
/// ```text
/// column([button("A1"), button("B2"), button("C3")])
/// height = 4
/// Tab, Tab, render
/// ```
///
/// # Assertions
///
/// - The second button receives focus.
/// - Rendering scrolls the column by the minimum amount needed.
/// - The focused button label is visible in the terminal buffer.
///
/// # Why
///
/// Keyboard focus should not move to an offscreen button without bringing that
/// button into view.
#[test]
fn tab_focus_scrolls_overflowing_column_to_focused_button() -> Result<()> {
    let width = 18;
    let backend = TestBackend::new(width, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column([button("A1"), button("B2"), button("C3")]);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(button_focuses(&view), vec![false, true, false]);
    assert_eq!(scroll_offset(&view), 2);
    assert!(symbol_position_opt(&terminal, "B", width).is_some());

    Ok(())
}

/// Verifies back-tab focus scrolls upward to a previously offscreen button.
///
/// # Example Under Test
///
/// ```text
/// column([button("A1"), button("B2"), button("C3")])
/// height = 4
/// Tab x3, render, BackTab, render, BackTab, render
/// ```
///
/// # Assertions
///
/// - Forward tabbing scrolls down to the third button.
/// - Back-tab to the second button scrolls just enough to reveal it.
/// - Back-tab to the first button returns to the top.
///
/// # Why
///
/// Reverse focus movement should use the same focus visibility rule as forward
/// movement.
#[test]
fn backtab_focus_scrolls_overflowing_column_up_to_focused_button() -> Result<()> {
    let width = 18;
    let backend = TestBackend::new(width, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column([button("A1"), button("B2"), button("C3")]);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;
    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 5);
    assert!(symbol_position_opt(&terminal, "C", width).is_some());

    view.handle_event(key(KeyCode::BackTab))?;
    draw_view(&mut terminal, &view)?;

    assert_eq!(button_focuses(&view), vec![false, true, false]);
    assert_eq!(scroll_offset(&view), 3);
    assert!(symbol_position_opt(&terminal, "B", width).is_some());

    view.handle_event(key(KeyCode::BackTab))?;
    draw_view(&mut terminal, &view)?;

    assert_eq!(button_focuses(&view), vec![true, false, false]);
    assert_eq!(scroll_offset(&view), 0);
    assert!(symbol_position_opt(&terminal, "A", width).is_some());

    Ok(())
}

/// Verifies focus scrolling does not pin later manual scroll movement.
///
/// # Example Under Test
///
/// ```text
/// column([button("A1"), button("B2"), button("C3")])
/// Tab, Tab, render, PageDown, render
/// ```
///
/// # Assertions
///
/// - Focus scrolling first reveals the second button.
/// - A later page-down scroll is preserved after rendering.
///
/// # Why
///
/// Automatic focus visibility should be a response to focus movement, not a
/// permanent constraint that prevents normal scrolling.
#[test]
fn focus_scroll_request_does_not_override_later_manual_scroll() -> Result<()> {
    let width = 18;
    let backend = TestBackend::new(width, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column([button("A1"), button("B2"), button("C3")]);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;
    draw_view(&mut terminal, &view)?;
    assert_eq!(scroll_offset(&view), 2);

    view.handle_event(key(KeyCode::PageDown))?;
    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 5);

    Ok(())
}

/// Verifies text-area editing scrolls an overflowing parent to the cursor.
///
/// # Example Under Test
///
/// ```text
/// column([text("Top"), focused text_area("one\ntwo\nthree\nfour"), text("Bottom")])
/// Enter, reconcile, render
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds with no parent scroll.
/// - Enter is handled by the focused text area.
/// - Reconciliation preserves editable state after the input callback updates.
/// - The next draw scrolls the parent to show the cursor.
/// - The terminal cursor lands on the expected row.
///
/// # Why
///
/// Controlled text-area edits can change child height and must keep the cursor
/// visible inside overflowing parents.
#[test]
fn text_area_editing_scrolls_overflowing_parent_to_cursor() -> Result<()> {
    let width = 12;
    let backend = TestBackend::new(width, 5);
    let mut terminal = Terminal::new(backend)?;
    let notes = Rc::new(RefCell::new(String::from("one\ntwo\nthree\nfour")));
    let build_view = |notes: &Rc<RefCell<String>>| {
        let value = notes.borrow().clone();
        let cursor = value.len();
        let notes_for_input = Rc::clone(notes);
        let mut notes_view = text_area(value).with_focus(true).on_input(move |next| {
            *notes_for_input.borrow_mut() = next;
            AppControl::Continue
        });
        editable_state_mut(&mut notes_view).set_mode(VimMode::Insert);
        editable_state_mut(&mut notes_view).set_cursor(cursor);

        column([text("Top"), notes_view, text("Bottom")])
    };
    let mut view = build_view(&notes);

    draw_view(&mut terminal, &view)?;
    assert_eq!(scroll_offset(&view), 0);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );

    let previous = view;
    let mut view = build_view(&notes);
    leptatui::__private::__reconcile_view(&mut view, &previous);
    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 2);
    terminal.backend_mut().assert_cursor_position((1, 4));

    Ok(())
}

/// Verifies normal-mode input boundary keys scroll an overflowing form.
///
/// # Example Under Test
///
/// ```text
/// form([text("Top"), focused normal-mode input("Ada"), trailing text rows])
/// j, k
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds with no form scroll.
/// - `j` is handled and scrolls the form down.
/// - `k` is handled and scrolls the form back to the top.
///
/// # Why
///
/// Single-line inputs at movement boundaries should pass normal-mode movement
/// intent to their overflowing parent form.
#[test]
fn normal_mode_input_boundary_keys_scroll_overflowing_form() -> Result<()> {
    let backend = TestBackend::new(12, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut input_view = input("Ada").with_focus(true);
    editable_state_mut(&mut input_view).set_mode(VimMode::Normal);
    let mut view = form([
        text("Top"),
        input_view,
        text("After 1"),
        text("After 2"),
        text("After 3"),
    ]);

    draw_view(&mut terminal, &view)?;
    assert_eq!(scroll_offset(&view), 0);

    assert_eq!(
        view.handle_event(key(KeyCode::Char('j')))?,
        AppControl::Continue
    );
    assert_eq!(scroll_offset(&view), 1);

    assert_eq!(
        view.handle_event(key(KeyCode::Char('k')))?,
        AppControl::Continue
    );
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}

/// Verifies normal-mode text-area boundary keys scroll an overflowing form.
///
/// # Example Under Test
///
/// ```text
/// form([text("Top"), focused normal-mode text_area("one\ntwo"), trailing text rows])
/// j, j, k, k
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds with no form scroll.
/// - The first `j` moves within the text area without parent scrolling.
/// - The second `j` is handled at the boundary and scrolls the form down.
/// - The first `k` moves within the text area without parent scrolling up.
/// - The second `k` is handled at the boundary and scrolls the form to the top.
///
/// # Why
///
/// Multi-line text areas should only delegate normal-mode movement to the form
/// after reaching their own vertical boundaries.
#[test]
fn normal_mode_text_area_boundary_keys_scroll_overflowing_form() -> Result<()> {
    let backend = TestBackend::new(12, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut text_area_view = text_area("one\ntwo").with_focus(true);
    editable_state_mut(&mut text_area_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut text_area_view).set_cursor(0);
    let mut view = form([
        text("Top"),
        text_area_view,
        text("After 1"),
        text("After 2"),
    ]);

    draw_view(&mut terminal, &view)?;
    assert_eq!(scroll_offset(&view), 0);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(form_child(&view, 1)).cursor(), 4);
    assert_eq!(scroll_offset(&view), 0);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(form_child(&view, 1)).cursor(), 4);
    assert_eq!(scroll_offset(&view), 1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(form_child(&view, 1)).cursor(), 0);
    assert_eq!(scroll_offset(&view), 1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(form_child(&view, 1)).cursor(), 0);
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}

/// Verifies focus scrolling works through component boundaries.
///
/// # Example Under Test
///
/// ```text
/// column([button("A1"), component(FocusPanel(button("B2"))), button("C3")])
/// height = 4
/// Tab, Tab, render
/// ```
///
/// # Assertions
///
/// - Tabbing into the component boundary succeeds.
/// - Rendering scrolls the parent column to the component's focused button.
/// - The component button label is visible in the terminal buffer.
///
/// # Why
///
/// Component boundaries should preserve the built-in focus visibility behavior.
#[test]
fn tab_focus_scrolls_to_focused_button_inside_component_boundary() -> Result<()> {
    let width = 18;
    let backend = TestBackend::new(width, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column([
        button("A1"),
        component(FocusPanel { view: button("B2") }),
        button("C3"),
    ]);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 2);
    assert!(symbol_position_opt(&terminal, "B", width).is_some());

    Ok(())
}

/// Verifies enter and space activate focused button actions.
///
/// # Example Under Test
///
/// ```text
/// column([button("Enter").on_press(...), button("Space").on_press(...)])
/// Tab, Enter, Tab, Space
/// ```
///
/// # Assertions
///
/// - The first tab event succeeds.
/// - The enter event succeeds and increments the action count to `1`.
/// - The second tab event succeeds.
/// - The space event succeeds and increments the action count to `2`.
///
/// # Why
///
/// Both common activation keys should trigger only the currently focused
/// button.
#[test]
fn enter_and_space_activate_focused_button() -> Result<()> {
    let count = Rc::new(Cell::new(0));
    let enter_count = Rc::clone(&count);
    let space_count = Rc::clone(&count);

    let mut view = column([
        button("Enter").on_press(move || {
            enter_count.set(enter_count.get() + 1);
            AppControl::Continue
        }),
        button("Space").on_press(move || {
            space_count.set(space_count.get() + 1);
            AppControl::Continue
        }),
    ]);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Enter))?;
    assert_eq!(count.get(), 1);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Char(' ')))?;
    assert_eq!(count.get(), 2);

    Ok(())
}

/// Verifies focused input character keys emit inserted text through `on_input`.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// Char('!'), Char(' ')
/// ```
///
/// # Assertions
///
/// - The `!` key is handled.
/// - The space key is handled.
/// - The callback receives `Ada!`.
/// - The callback receives `Ada `.
#[test]
fn focused_input_emits_inserted_text_through_on_input() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_char = Rc::clone(&emitted);
    let mut char_view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_char.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut char_view).set_mode(VimMode::Insert);

    assert_eq!(
        char_view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_space = Rc::clone(&emitted);
    let mut space_view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_space.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut space_view).set_mode(VimMode::Insert);

    assert_eq!(
        space_view.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(
        emitted.borrow().as_slice(),
        &[String::from("Ada!"), String::from("Ada ")]
    );

    Ok(())
}

/// Verifies focused inputs leave insert mode on the `jk` key sequence.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// j, k
/// ```
///
/// # Assertions
///
/// - Both keys are handled.
/// - No input value is emitted.
/// - The input switches to normal mode with Esc-style cursor placement.
#[test]
fn focused_input_jk_returns_to_normal_mode_without_emitting_text() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("Ada", &emitted);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Insert);
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).cursor(), 2);
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    Ok(())
}

/// Verifies a pending insert-mode `j` is inserted with the next non-escape key.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// j, x
/// ```
///
/// # Assertions
///
/// - The first `j` waits for the next key.
/// - The following `x` emits both inserted characters.
/// - The input remains in insert mode.
#[test]
fn focused_input_pending_j_inserts_with_next_non_escape_character() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("Ada", &emitted);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('x')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&view).cursor(), 5);
    assert_eq!(emitted.borrow().as_slice(), &[String::from("Adajx")]);

    Ok(())
}

/// Verifies slow insert-mode `jk` is inserted as literal text.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// j, wait past timeout, k
/// ```
///
/// # Assertions
///
/// - The first `j` waits for the next key.
/// - The later `k` emits literal `jk`.
/// - The input remains in insert mode.
#[test]
fn focused_input_slow_jk_inserts_literal_text() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("Ada", &emitted);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    thread::sleep(Duration::from_millis(1100));

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&view).cursor(), 5);
    assert_eq!(emitted.borrow().as_slice(), &[String::from("Adajk")]);

    Ok(())
}

/// Verifies an expired pending insert-mode `j` is emitted without another key.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// j, wait past timeout, flush
/// ```
///
/// # Assertions
///
/// - The first `j` waits for the timeout.
/// - Flushing emits literal `j`.
/// - A second flush has nothing to emit.
#[test]
fn focused_input_idle_flush_emits_expired_pending_j() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("Ada", &emitted);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    thread::sleep(Duration::from_millis(1100));

    assert_eq!(view.__flush_pending_input(), Some(AppControl::Continue));
    assert_eq!(editable_state(&view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&view).cursor(), 4);
    assert_eq!(emitted.borrow().as_slice(), &[String::from("Adaj")]);
    assert_eq!(view.__flush_pending_input(), None);

    Ok(())
}

/// Verifies focused input deletion keys emit shortened text through `on_input`.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// Backspace, Delete at cursor 1
/// ```
///
/// # Assertions
///
/// - The backspace key is handled.
/// - The delete key is handled.
/// - The callback receives `Ad` after backspace.
/// - The callback receives `Aa` after delete.
#[test]
fn focused_input_emits_deletions_through_on_input() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_backspace = Rc::clone(&emitted);
    let mut backspace_view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_backspace.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut backspace_view).set_mode(VimMode::Insert);

    assert_eq!(
        backspace_view.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_delete = Rc::clone(&emitted);
    let mut delete_view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_delete.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut delete_view).set_mode(VimMode::Insert);
    editable_state_mut(&mut delete_view).set_cursor(1);

    assert_eq!(
        delete_view.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(
        emitted.borrow().as_slice(),
        &[String::from("Ad"), String::from("Aa")]
    );

    Ok(())
}

/// Verifies focused input cursor keys move without emitting text.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// Left, Home, Right, End
/// ```
///
/// # Assertions
///
/// - Left moves the cursor to byte index `2`.
/// - Home moves the cursor to byte index `0`.
/// - Right moves the cursor to byte index `1`.
/// - End moves the cursor to byte index `3`.
/// - No input callback values are emitted.
#[test]
fn focused_input_cursor_keys_move_without_emitting_text() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_input = Rc::clone(&emitted);
    let mut view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_input.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut view).set_mode(VimMode::Insert);

    view.handle_key_event(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 2);

    view.handle_key_event(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    view.handle_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 1);

    view.handle_key_event(KeyEvent::new(KeyCode::End, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 3);
    assert!(emitted.borrow().is_empty());

    Ok(())
}

/// Verifies focused inputs without callbacks do not mutate displayed values.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// Char('!')
/// ```
///
/// # Assertions
///
/// - The character key is handled.
/// - The retained input value remains `Ada`.
/// - Rendering still shows `Ada`.
/// - The cell after the value remains blank.
#[test]
fn focused_input_without_callback_does_not_mutate_displayed_value() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("Ada").with_focus(true);

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    match &view {
        View::Input { value, .. } => assert_eq!(value, "Ada"),
        other => panic!("expected input view, got {other:?}"),
    }

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "A");
    assert_eq!(cell_symbol(&terminal, 3, 1, 8), "a");
    assert_eq!(cell_symbol(&terminal, 4, 1, 8), " ");

    Ok(())
}

/// Verifies focused text-area insertion keys emit full next values.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\nLovelace").with_focus(true).on_input(...)
/// Char('!'), Enter
/// ```
///
/// # Assertions
///
/// - The character key is handled.
/// - The enter key is handled.
/// - The callbacks receive the full proposed multiline values.
#[test]
fn focused_text_area_emits_inserted_text_through_on_input() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_char = Rc::clone(&emitted);
    let mut char_view = text_area("Ada\nLovelace")
        .with_focus(true)
        .on_input(move |next| {
            emitted_for_char.borrow_mut().push(next);
            AppControl::Continue
        });
    editable_state_mut(&mut char_view).set_mode(VimMode::Insert);

    assert_eq!(
        char_view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_enter = Rc::clone(&emitted);
    let mut enter_view = text_area("Ada").with_focus(true).on_input(move |next| {
        emitted_for_enter.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut enter_view).set_mode(VimMode::Insert);

    assert_eq!(
        enter_view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(
        emitted.borrow().as_slice(),
        &[String::from("Ada\nLovelace!"), String::from("Ada\n")]
    );

    Ok(())
}

/// Verifies focused text areas leave insert mode on the `jk` key sequence.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\nLovelace").with_focus(true).on_input(...)
/// j, k
/// ```
///
/// # Assertions
///
/// - Both keys are handled.
/// - No input value is emitted.
/// - The text area switches to normal mode with Esc-style cursor placement.
#[test]
fn focused_text_area_jk_returns_to_normal_mode_without_emitting_text() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_text_area("Ada\nLovelace", &emitted);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Insert);
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).cursor(), 11);
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    Ok(())
}

/// Verifies slow text-area insert-mode `jk` is inserted as literal text.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\nLovelace").with_focus(true).on_input(...)
/// j, wait past timeout, k
/// ```
///
/// # Assertions
///
/// - The first `j` waits for the next key.
/// - The later `k` emits literal `jk`.
/// - The text area remains in insert mode.
#[test]
fn focused_text_area_slow_jk_inserts_literal_text() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_text_area("Ada\nLovelace", &emitted);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    thread::sleep(Duration::from_millis(1100));

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&view).cursor(), 14);
    assert_eq!(
        emitted.borrow().as_slice(),
        &[String::from("Ada\nLovelacejk")]
    );

    Ok(())
}

/// Verifies focused text-area deletion keys can remove line boundaries.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\nLovelace").with_focus(true).on_input(...)
/// Backspace after newline, Delete before newline
/// ```
///
/// # Assertions
///
/// - Backspace at the start of the second line is handled.
/// - Delete at the end of the first line is handled.
/// - Both callbacks receive the joined multiline value.
#[test]
fn focused_text_area_emits_line_boundary_deletions_through_on_input() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_backspace = Rc::clone(&emitted);
    let mut backspace_view = text_area("Ada\nLovelace")
        .with_focus(true)
        .on_input(move |next| {
            emitted_for_backspace.borrow_mut().push(next);
            AppControl::Continue
        });
    editable_state_mut(&mut backspace_view).set_mode(VimMode::Insert);
    editable_state_mut(&mut backspace_view).set_cursor(4);

    assert_eq!(
        backspace_view.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_delete = Rc::clone(&emitted);
    let mut delete_view = text_area("Ada\nLovelace")
        .with_focus(true)
        .on_input(move |next| {
            emitted_for_delete.borrow_mut().push(next);
            AppControl::Continue
        });
    editable_state_mut(&mut delete_view).set_mode(VimMode::Insert);
    editable_state_mut(&mut delete_view).set_cursor(3);

    assert_eq!(
        delete_view.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(
        emitted.borrow().as_slice(),
        &[String::from("AdaLovelace"), String::from("AdaLovelace")]
    );

    Ok(())
}

/// Verifies focused text-area cursor keys move without emitting values.
///
/// # Example Under Test
///
/// ```text
/// text_area("abc\nde\nfghi").with_focus(true).on_input(...)
/// Up, Up, Down, Down, Home, End
/// ```
///
/// # Assertions
///
/// - Up and down move between logical lines at the nearest available column.
/// - Home and End move within the current logical line.
/// - No input callback values are emitted.
#[test]
fn focused_text_area_cursor_keys_move_without_emitting_text() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_text_area = Rc::clone(&emitted);
    let mut view = text_area("abc\nde\nfghi")
        .with_focus(true)
        .on_input(move |next| {
            emitted_for_text_area.borrow_mut().push(next);
            AppControl::Continue
        });
    editable_state_mut(&mut view).set_mode(VimMode::Insert);
    editable_state_mut(&mut view).set_cursor(9);

    view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 6);

    view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 2);

    view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 6);

    view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 9);

    view.handle_key_event(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 7);

    view.handle_key_event(KeyEvent::new(KeyCode::End, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 11);
    assert!(emitted.borrow().is_empty());

    Ok(())
}

/// Verifies editable controls support Vim mode transition keys.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// i, Esc, a, I, A
///
/// text_area("ab\ncd").with_focus(true)
/// I, A
/// ```
///
/// # Assertions
///
/// - Inputs start in normal mode.
/// - Esc from insert mode switches the input to normal mode and moves the cursor onto the
///   previous character.
/// - `i` and `a` switch the input to insert mode at the current and next
///   normal-mode positions.
/// - `I` and `A` move to the line start and line end for inputs and text
///   areas.
#[test]
fn focused_editable_controls_support_vim_mode_transitions() -> Result<()> {
    let mut input_view = input("Ada").with_focus(true);
    assert_eq!(editable_state(&input_view).mode(), VimMode::Normal);

    editable_state_mut(&mut input_view).set_mode(VimMode::Insert);
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&input_view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&input_view).cursor(), 2);

    editable_state_mut(&mut input_view).set_cursor(1);
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Char('i')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&input_view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&input_view).cursor(), 1);

    editable_state_mut(&mut input_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut input_view).set_cursor(1);
    input_view.handle_key_event(key_event(KeyCode::Char('a')))?;
    assert_eq!(editable_state(&input_view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&input_view).cursor(), 2);

    editable_state_mut(&mut input_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut input_view).set_cursor(2);
    input_view.handle_key_event(key_event(KeyCode::Char('I')))?;
    assert_eq!(editable_state(&input_view).cursor(), 0);

    editable_state_mut(&mut input_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut input_view).set_cursor(0);
    input_view.handle_key_event(key_event(KeyCode::Char('A')))?;
    assert_eq!(editable_state(&input_view).cursor(), 3);

    let mut text_area_view = text_area("ab\ncd").with_focus(true);
    editable_state_mut(&mut text_area_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut text_area_view).set_cursor(4);
    text_area_view.handle_key_event(key_event(KeyCode::Char('I')))?;
    assert_eq!(editable_state(&text_area_view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&text_area_view).cursor(), 3);

    editable_state_mut(&mut text_area_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut text_area_view).set_cursor(4);
    text_area_view.handle_key_event(key_event(KeyCode::Char('A')))?;
    assert_eq!(editable_state(&text_area_view).cursor(), 5);

    Ok(())
}

/// Verifies focused text areas support Vim normal-mode open-line commands.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo").with_focus(true).on_input(...)
/// o, O
///
/// text_area("").with_focus(true).on_input(...)
/// o, O
/// ```
///
/// # Assertions
///
/// - `o` opens a blank line below the current logical line, enters insert mode,
///   and places the cursor on that blank line.
/// - `O` opens a blank line above the current logical line, enters insert mode,
///   and places the cursor on that blank line.
/// - Opening below the final line appends a trailing blank line.
/// - Opening above the first line prepends a blank line.
/// - Empty text areas enter insert mode without emitting a changed value.
#[test]
fn focused_text_area_supports_vim_open_line_commands() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut below_middle = emitting_text_area("one\ntwo", &emitted);
    editable_state_mut(&mut below_middle).set_mode(VimMode::Normal);
    editable_state_mut(&mut below_middle).set_cursor(1);

    assert_eq!(
        below_middle.handle_key_event(key_event(KeyCode::Char('o')))?,
        KeyControl::Handled
    );
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\n\ntwo")
    );
    assert_eq!(editable_state(&below_middle).mode(), VimMode::Insert);
    assert_eq!(editable_state(&below_middle).cursor(), 4);
    assert_eq!(
        editable_state(&below_middle).undo_stack(),
        &[String::from("one\ntwo")]
    );

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut above_middle = emitting_text_area("one\ntwo", &emitted);
    editable_state_mut(&mut above_middle).set_mode(VimMode::Normal);
    editable_state_mut(&mut above_middle).set_cursor(5);

    above_middle.handle_key_event(key_event(KeyCode::Char('O')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\n\ntwo")
    );
    assert_eq!(editable_state(&above_middle).mode(), VimMode::Insert);
    assert_eq!(editable_state(&above_middle).cursor(), 4);

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut below_final = emitting_text_area("one\ntwo", &emitted);
    editable_state_mut(&mut below_final).set_mode(VimMode::Normal);
    editable_state_mut(&mut below_final).set_cursor(4);

    below_final.handle_key_event(key_event(KeyCode::Char('o')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\ntwo\n")
    );
    assert_eq!(editable_state(&below_final).mode(), VimMode::Insert);
    assert_eq!(editable_state(&below_final).cursor(), 8);

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut above_first = emitting_text_area("one\ntwo", &emitted);
    editable_state_mut(&mut above_first).set_mode(VimMode::Normal);
    editable_state_mut(&mut above_first).set_cursor(1);

    above_first.handle_key_event(key_event(KeyCode::Char('O')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("\none\ntwo")
    );
    assert_eq!(editable_state(&above_first).mode(), VimMode::Insert);
    assert_eq!(editable_state(&above_first).cursor(), 0);

    above_first = reconcile_text_area_value(&above_first, "\none\ntwo", &emitted);
    assert_eq!(
        above_first.handle_key_event(key_event(KeyCode::Backspace))?,
        KeyControl::Handled
    );
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\ntwo")
    );
    assert_eq!(editable_state(&above_first).mode(), VimMode::Insert);
    assert_eq!(editable_state(&above_first).cursor(), 0);

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut empty_below = emitting_text_area("", &emitted);
    editable_state_mut(&mut empty_below).set_mode(VimMode::Normal);

    empty_below.handle_key_event(key_event(KeyCode::Char('o')))?;
    assert!(emitted.borrow().is_empty());
    assert_eq!(editable_state(&empty_below).mode(), VimMode::Insert);
    assert_eq!(editable_state(&empty_below).cursor(), 0);

    let mut empty_above = emitting_text_area("", &emitted);
    editable_state_mut(&mut empty_above).set_mode(VimMode::Normal);

    empty_above.handle_key_event(key_event(KeyCode::Char('O')))?;
    assert!(emitted.borrow().is_empty());
    assert_eq!(editable_state(&empty_above).mode(), VimMode::Insert);
    assert_eq!(editable_state(&empty_above).cursor(), 0);

    Ok(())
}

/// Verifies focused inputs handle Vim open-line keys as no-ops.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// o, O
/// ```
///
/// # Assertions
///
/// - `o` and `O` are handled so they do not leak to parent key handling.
/// - Inputs do not emit values or leave normal mode for multiline-only
///   open-line commands.
#[test]
fn focused_input_handles_vim_open_line_commands_without_mutation() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("Ada", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('o')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).cursor(), 1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('O')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).cursor(), 1);
    assert!(emitted.borrow().is_empty());

    Ok(())
}

/// Verifies focused inputs support Vim normal-mode movement.
///
/// # Example Under Test
///
/// ```text
/// input("one two three").with_focus(true)
/// l, Left, Right, h, w, e, b, $, 0, G, gg
/// ```
///
/// # Assertions
///
/// - Character movement keys and arrows update the input cursor.
/// - Word motions move to the expected word start and end positions.
/// - Line and value boundary motions move to the first or last character.
/// - `gg` moves the cursor back to the first character.
#[test]
fn focused_input_supports_vim_normal_mode_movement() -> Result<()> {
    let mut view = input("one two three").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(0);

    view.handle_key_event(key_event(KeyCode::Char('l')))?;
    assert_eq!(editable_state(&view).cursor(), 1);

    view.handle_key_event(key_event(KeyCode::Left))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    view.handle_key_event(key_event(KeyCode::Right))?;
    assert_eq!(editable_state(&view).cursor(), 1);

    view.handle_key_event(key_event(KeyCode::Char('h')))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    view.handle_key_event(key_event(KeyCode::Char('w')))?;
    assert_eq!(editable_state(&view).cursor(), 4);

    view.handle_key_event(key_event(KeyCode::Char('e')))?;
    assert_eq!(editable_state(&view).cursor(), 6);

    view.handle_key_event(key_event(KeyCode::Char('b')))?;
    assert_eq!(editable_state(&view).cursor(), 4);

    view.handle_key_event(key_event(KeyCode::Char('$')))?;
    assert_eq!(editable_state(&view).cursor(), 12);

    view.handle_key_event(key_event(KeyCode::Char('0')))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    view.handle_key_event(key_event(KeyCode::Char('G')))?;
    assert_eq!(editable_state(&view).cursor(), 12);

    view.handle_key_event(key_event(KeyCode::Char('g')))?;
    view.handle_key_event(key_event(KeyCode::Char('g')))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    Ok(())
}

/// Verifies focused text areas support Vim normal-mode movement.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo\nthree").with_focus(true)
/// j, k, Down, Up, $, 0, G, gg
/// ```
///
/// # Assertions
///
/// - `j`, `k`, Down, and Up move between logical lines.
/// - Vertical movement preserves the nearest available column.
/// - `$` and `0` move to the current line end and start.
/// - `G` and `gg` move to the last and first characters in the text area.
#[test]
fn focused_text_area_supports_vim_normal_mode_movement() -> Result<()> {
    let mut view = text_area("one\ntwo\nthree").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(4);

    view.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(editable_state(&view).cursor(), 8);

    view.handle_key_event(key_event(KeyCode::Char('k')))?;
    assert_eq!(editable_state(&view).cursor(), 4);

    view.handle_key_event(key_event(KeyCode::Down))?;
    assert_eq!(editable_state(&view).cursor(), 8);

    view.handle_key_event(key_event(KeyCode::Up))?;
    assert_eq!(editable_state(&view).cursor(), 4);

    editable_state_mut(&mut view).set_cursor(5);
    view.handle_key_event(key_event(KeyCode::Char('k')))?;
    assert_eq!(editable_state(&view).cursor(), 1);

    editable_state_mut(&mut view).set_cursor(8);
    view.handle_key_event(key_event(KeyCode::Char('$')))?;
    assert_eq!(editable_state(&view).cursor(), 12);

    view.handle_key_event(key_event(KeyCode::Char('0')))?;
    assert_eq!(editable_state(&view).cursor(), 8);

    view.handle_key_event(key_event(KeyCode::Char('G')))?;
    assert_eq!(editable_state(&view).cursor(), 12);

    view.handle_key_event(key_event(KeyCode::Char('g')))?;
    view.handle_key_event(key_event(KeyCode::Char('g')))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    Ok(())
}

/// Verifies focused text areas keep trailing blank lines reachable in normal mode.
#[test]
fn focused_text_area_supports_trailing_blank_line_normal_mode_movement() -> Result<()> {
    let value = "one\ntwo\n";
    let trailing_blank_cursor = value.len();
    let mut view = text_area(value).with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Insert);
    editable_state_mut(&mut view).set_cursor(trailing_blank_cursor);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).cursor(), trailing_blank_cursor);

    view.handle_key_event(key_event(KeyCode::Char('k')))?;
    assert_eq!(editable_state(&view).cursor(), 4);

    view.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(editable_state(&view).cursor(), trailing_blank_cursor);

    Ok(())
}

/// Verifies focused inputs support Vim character-wise visual mode transitions.
///
/// # Example Under Test
///
/// ```text
/// input("abcd").with_focus(true)
/// v, l, h, Esc
/// ```
///
/// # Assertions
///
/// - `v` enters character-wise visual mode and anchors at the current cursor.
/// - Normal movement keys move the cursor while preserving the anchor.
/// - Esc returns to normal mode and clears the selection anchor.
#[test]
fn focused_input_supports_vim_visual_mode_transitions() -> Result<()> {
    let mut view = input("abcd").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('v')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Visual);
    assert_eq!(editable_state(&view).selection_anchor(), Some(1));

    view.handle_key_event(key_event(KeyCode::Char('l')))?;
    assert_eq!(editable_state(&view).cursor(), 2);
    assert_eq!(editable_state(&view).selection_anchor(), Some(1));

    view.handle_key_event(key_event(KeyCode::Char('h')))?;
    assert_eq!(editable_state(&view).cursor(), 1);
    assert_eq!(editable_state(&view).selection_anchor(), Some(1));

    view.handle_key_event(key_event(KeyCode::Esc))?;
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).selection_anchor(), None);

    Ok(())
}

/// Verifies character-wise visual yank and delete commands use the selection.
///
/// # Example Under Test
///
/// ```text
/// input("abcd").with_focus(true).on_input(...)
/// v, l, y, then v, l, d
/// ```
///
/// # Assertions
///
/// - `y` yanks the selected characters and exits visual mode.
/// - `d` deletes the selected characters, emits the controlled value, and
///   records undo history.
#[test]
fn focused_input_supports_visual_yank_and_delete() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("abcd", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(1);

    view.handle_key_event(key_event(KeyCode::Char('v')))?;
    view.handle_key_event(key_event(KeyCode::Char('l')))?;
    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).selection_anchor(), None);
    assert_eq!(editable_state(&view).yank_buffer(), "bc");
    assert_eq!(editable_state(&view).cursor(), 1);

    view.handle_key_event(key_event(KeyCode::Char('v')))?;
    view.handle_key_event(key_event(KeyCode::Char('l')))?;
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('d')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("ad"));
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).selection_anchor(), None);
    assert_eq!(editable_state(&view).yank_buffer(), "bc");
    assert_eq!(editable_state(&view).undo_stack(), &[String::from("abcd")]);

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("abcd", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(1);
    view.handle_key_event(key_event(KeyCode::Char('v')))?;
    view.handle_key_event(key_event(KeyCode::Char('l')))?;
    view.handle_key_event(key_event(KeyCode::Char('x')))?;
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("ad"));
    assert_eq!(editable_state(&view).yank_buffer(), "bc");

    Ok(())
}

/// Verifies visual-line text-area yank, paste, and delete work linewise.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo\nthree").with_focus(true).on_input(...)
/// V, j, y, G, p
/// V, j, d
/// ```
///
/// # Assertions
///
/// - Visual-line `y` stores selected logical lines in the linewise yank buffer.
/// - `p` pastes the linewise selection below the current line.
/// - Visual-line `d` removes all selected logical lines and records undo
///   history.
#[test]
fn focused_text_area_supports_visual_line_yank_paste_and_delete() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_text_area("one\ntwo\nthree", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(4);

    view.handle_key_event(key_event(KeyCode::Char('V')))?;
    view.handle_key_event(key_event(KeyCode::Char('j')))?;
    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).selection_anchor(), None);
    assert_eq!(editable_state(&view).yank_buffer(), "two\nthree");

    view.handle_key_event(key_event(KeyCode::Char('G')))?;
    view.handle_key_event(key_event(KeyCode::Char('p')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\ntwo\nthree\ntwo\nthree")
    );

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_text_area("one\ntwo\nthree", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(4);

    view.handle_key_event(key_event(KeyCode::Char('V')))?;
    view.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('d')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("one"));
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).selection_anchor(), None);
    assert_eq!(editable_state(&view).yank_buffer(), "two\nthree");
    assert_eq!(
        editable_state(&view).undo_stack(),
        &[String::from("one\ntwo\nthree")]
    );

    Ok(())
}

/// Verifies focused inputs support normal-mode mutation and history commands.
///
/// # Example Under Test
///
/// ```text
/// input("abc").with_focus(true).on_input(...)
/// x, yy, p, dd, u, Ctrl+r
/// ```
///
/// # Assertions
///
/// - `x` emits `ac` and records the original value in undo history.
/// - `yy` yanks the current input value.
/// - `p` emits the pasted `acac` value.
/// - `dd` emits an empty value.
/// - `u` emits the previous value and records redo history.
/// - Ctrl+r emits the redone empty value.
/// - The full emitted value sequence matches the expected mutation order.
///
/// # Why
///
/// Undo and redo history must survive controlled-value reconciliation between
/// emitted input values.
#[test]
fn focused_input_supports_vim_delete_yank_paste_undo_and_redo() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("abc", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('x')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("ac"));
    assert_eq!(editable_state(&view).undo_stack(), &[String::from("abc")]);

    view = reconcile_input_value(&view, "ac", &emitted);
    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    assert_eq!(editable_state(&view).yank_buffer(), "ac");

    view.handle_key_event(key_event(KeyCode::Char('p')))?;
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("acac"));

    view = reconcile_input_value(&view, "acac", &emitted);
    view.handle_key_event(key_event(KeyCode::Char('d')))?;
    view.handle_key_event(key_event(KeyCode::Char('d')))?;
    assert_eq!(emitted.borrow().last().map(String::as_str), Some(""));

    view = reconcile_input_value(&view, "", &emitted);
    view.handle_key_event(key_event(KeyCode::Char('u')))?;
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("acac"));
    assert_eq!(editable_state(&view).redo_stack(), &[String::new()]);

    view = reconcile_input_value(&view, "acac", &emitted);
    view.handle_key_event(ctrl_key_event('r'))?;
    assert_eq!(emitted.borrow().last().map(String::as_str), Some(""));

    assert_eq!(
        emitted.borrow().as_slice(),
        &[
            String::from("ac"),
            String::from("acac"),
            String::new(),
            String::from("acac"),
            String::new(),
        ]
    );

    Ok(())
}

/// Verifies focused text areas support linewise yank, delete, paste, and history.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo\nthree").with_focus(true).on_input(...)
/// yy, G, p, dd, u, Ctrl+r
/// ```
///
/// # Assertions
///
/// - `yy` yanks the current logical line without a trailing newline.
/// - `p` after `G` appends the yanked line below the final line.
/// - `dd` deletes the selected logical line and keeps that line in the yank
///   buffer.
/// - `u` emits the previous text-area value.
/// - Ctrl+r emits the redone line-deleted value.
///
/// # Why
///
/// Linewise operations need different paste ranges than character-wise input
/// operations.
#[test]
fn focused_text_area_supports_linewise_yank_delete_paste_undo_and_redo() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_text_area("one\ntwo\nthree", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(4);

    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    assert_eq!(editable_state(&view).yank_buffer(), "two");

    view.handle_key_event(key_event(KeyCode::Char('G')))?;
    view.handle_key_event(key_event(KeyCode::Char('p')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\ntwo\nthree\ntwo")
    );

    view = reconcile_text_area_value(&view, "one\ntwo\nthree\ntwo", &emitted);
    editable_state_mut(&mut view).set_cursor(4);
    view.handle_key_event(key_event(KeyCode::Char('d')))?;
    view.handle_key_event(key_event(KeyCode::Char('d')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\nthree\ntwo")
    );
    assert_eq!(editable_state(&view).yank_buffer(), "two");

    view = reconcile_text_area_value(&view, "one\nthree\ntwo", &emitted);
    view.handle_key_event(key_event(KeyCode::Char('u')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\ntwo\nthree\ntwo")
    );

    view = reconcile_text_area_value(&view, "one\ntwo\nthree\ntwo", &emitted);
    view.handle_key_event(ctrl_key_event('r'))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\nthree\ntwo")
    );

    Ok(())
}

/// Verifies insert-mode Enter keeps inputs single-line and text areas multiline.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// Enter
///
/// text_area("Ada").with_focus(true).on_input(...)
/// Enter
/// ```
///
/// # Assertions
///
/// - Enter is handled for a focused input without emitting values.
/// - Enter is handled for a focused text area.
/// - The text-area callback emits the value with a trailing newline.
#[test]
fn insert_mode_keeps_input_single_line_and_text_area_multiline() -> Result<()> {
    let input_emitted = Rc::new(RefCell::new(Vec::new()));
    let mut input_view = emitting_input("Ada", &input_emitted);
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert!(input_emitted.borrow().is_empty());

    let text_area_emitted = Rc::new(RefCell::new(Vec::new()));
    let mut text_area_view = emitting_text_area("Ada", &text_area_emitted);
    assert_eq!(
        text_area_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(
        text_area_emitted.borrow().as_slice(),
        &[String::from("Ada\n")]
    );

    Ok(())
}

/// Verifies focused inputs submit forms on Enter in insert and normal mode.
///
/// # Example Under Test
///
/// ```text
/// form([input("Ada").with_focus(true)]).on_submit(...)
/// Enter
/// ```
///
/// # Assertions
///
/// - Insert-mode Enter is handled.
/// - Insert-mode Enter invokes the submit callback once.
/// - Normal-mode Enter is handled.
/// - Normal-mode Enter invokes the submit callback once.
#[test]
fn form_submits_focused_input_on_enter_in_insert_and_normal_mode() -> Result<()> {
    let insert_submits = Rc::new(Cell::new(0));
    let insert_submits_for_form = Rc::clone(&insert_submits);
    let mut insert_input = input("Ada").with_focus(true);
    editable_state_mut(&mut insert_input).set_mode(VimMode::Insert);
    let mut insert_view = form([insert_input]).on_submit(move || {
        insert_submits_for_form.set(insert_submits_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        insert_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(insert_submits.get(), 1);

    let normal_submits = Rc::new(Cell::new(0));
    let normal_submits_for_form = Rc::clone(&normal_submits);
    let mut normal_input = input("Ada").with_focus(true);
    editable_state_mut(&mut normal_input).set_mode(VimMode::Normal);
    let mut normal_view = form([normal_input]).on_submit(move || {
        normal_submits_for_form.set(normal_submits_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        normal_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(normal_submits.get(), 1);

    Ok(())
}

/// Verifies text areas keep multiline Enter behavior inside forms.
///
/// # Example Under Test
///
/// ```text
/// form([text_area("Ada").with_focus(true).on_input(...)])
/// Enter, Ctrl+Enter
/// ```
///
/// # Assertions
///
/// - Plain Enter is handled by the text area.
/// - Plain Enter emits a value with a trailing newline.
/// - Plain Enter does not submit the form.
/// - Ctrl+Enter is handled by the form.
/// - Ctrl+Enter submits the form without emitting another input value.
#[test]
fn form_text_area_uses_plain_enter_for_newlines_and_ctrl_enter_for_submit() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let submits = Rc::new(Cell::new(0));
    let submits_for_form = Rc::clone(&submits);
    let mut view = form([emitting_text_area("Ada", &emitted)]).on_submit(move || {
        submits_for_form.set(submits_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().as_slice(), &[String::from("Ada\n")]);
    assert_eq!(submits.get(), 0);

    assert_eq!(
        view.handle_key_event(ctrl_enter_key_event())?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().len(), 1);
    assert_eq!(submits.get(), 1);

    Ok(())
}

/// Verifies Esc leaves editable insert mode before canceling a form.
///
/// # Example Under Test
///
/// ```text
/// form([input("Ada").with_focus(true)]).on_cancel(...)
/// Esc, Esc
/// ```
///
/// # Assertions
///
/// - The first Esc is handled by the focused input.
/// - The first Esc does not invoke the cancel callback.
/// - The second Esc is handled by the form.
/// - The second Esc invokes the cancel callback once.
#[test]
fn form_esc_leaves_insert_mode_before_canceling() -> Result<()> {
    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut input_view = input("Ada").with_focus(true);
    editable_state_mut(&mut input_view).set_mode(VimMode::Insert);
    let mut view = form([input_view]).on_cancel(move || {
        cancels_for_form.set(cancels_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 0);
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 1);

    Ok(())
}

/// Verifies `jk` leaves editable insert mode before Esc cancels a form.
///
/// # Example Under Test
///
/// ```text
/// form([input("Ada").with_focus(true)]).on_cancel(...)
/// j, k, Esc
/// ```
///
/// # Assertions
///
/// - The `jk` sequence is handled by the focused input.
/// - The `jk` sequence does not invoke the cancel callback.
/// - A later Esc is handled by the form.
#[test]
fn form_jk_leaves_insert_mode_without_canceling() -> Result<()> {
    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut input_view = input("Ada").with_focus(true);
    editable_state_mut(&mut input_view).set_mode(VimMode::Insert);
    let mut view = form([input_view]).on_cancel(move || {
        cancels_for_form.set(cancels_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 0);
    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Normal);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 1);

    Ok(())
}

/// Verifies Esc leaves editable visual modes before canceling a form.
///
/// # Example Under Test
///
/// ```text
/// form([input("Ada").with_focus(true)]).on_cancel(...)
/// v, Esc, Esc
///
/// form([text_area("one\ntwo").with_focus(true)]).on_cancel(...)
/// V, Esc, Esc
/// ```
///
/// # Assertions
///
/// - The first Esc is handled by the focused editable visual mode.
/// - The first Esc does not invoke the cancel callback.
/// - The second Esc is handled by the form and invokes cancel.
#[test]
fn form_esc_leaves_visual_modes_before_canceling() -> Result<()> {
    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut input_view = input("Ada").with_focus(true);
    editable_state_mut(&mut input_view).set_mode(VimMode::Visual);
    editable_state_mut(&mut input_view).set_selection_anchor(Some(0));
    let mut view = form([input_view]).on_cancel(move || {
        cancels_for_form.set(cancels_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 0);
    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Normal);
    assert_eq!(
        editable_state(form_child(&view, 0)).selection_anchor(),
        None
    );

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 1);

    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut text_area_view = text_area("one\ntwo").with_focus(true);
    editable_state_mut(&mut text_area_view).set_cursor(4);
    editable_state_mut(&mut text_area_view).set_mode(VimMode::VisualLine);
    editable_state_mut(&mut text_area_view).set_selection_anchor(Some(4));
    let mut view = form([text_area_view]).on_cancel(move || {
        cancels_for_form.set(cancels_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 0);
    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Normal);
    assert_eq!(
        editable_state(form_child(&view, 0)).selection_anchor(),
        None
    );

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 1);

    Ok(())
}

/// Verifies forms inside component boundaries handle submit keys.
///
/// # Example Under Test
///
/// ```text
/// component(FocusPanel { view: form([focused input]).on_submit(...) })
/// Enter
/// ```
///
/// # Assertions
///
/// - Enter is handled through the component boundary.
/// - The nested form submit callback runs once.
#[test]
fn form_inside_component_boundary_handles_submit_key() -> Result<()> {
    let submits = Rc::new(Cell::new(0));
    let submits_for_form = Rc::clone(&submits);
    let view = form([input("Ada").with_focus(true)]).on_submit(move || {
        submits_for_form.set(submits_for_form.get() + 1);
        AppControl::Continue
    });
    let mut view = component(FocusPanel { view });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(submits.get(), 1);

    Ok(())
}

/// Verifies focused text areas without callbacks do not mutate displayed values.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\nLovelace").with_focus(true)
/// Char('!')
/// ```
///
/// # Assertions
///
/// - The character key is handled.
/// - The retained text-area value remains unchanged.
/// - Rendering still shows the original value.
/// - The cell after the first line remains blank.
#[test]
fn focused_text_area_without_callback_does_not_mutate_displayed_value() -> Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = text_area("Ada\nLovelace").with_focus(true);

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    match &view {
        View::TextArea { value, .. } => assert_eq!(value, "Ada\nLovelace"),
        other => panic!("expected text-area view, got {other:?}"),
    }

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 12), "A");
    assert_eq!(cell_symbol(&terminal, 3, 1, 12), "a");
    assert_eq!(cell_symbol(&terminal, 4, 1, 12), " ");

    Ok(())
}

/// Verifies focused input editing works inside component boundaries.
///
/// # Example Under Test
///
/// ```text
/// component(FocusPanel { view: input("Ada").on_input(...) })
/// Tab, A, Char('!')
/// ```
///
/// # Assertions
///
/// - Tabbing into the component boundary succeeds.
/// - The character key is handled by the focused input.
/// - The callback receives `Ada!`.
#[test]
fn focused_input_inside_component_boundary_handles_editing_keys() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_input = Rc::clone(&emitted);
    let input_view = input("Ada").on_input(move |next| {
        emitted_for_input.borrow_mut().push(next);
        AppControl::Continue
    });
    let mut view = component(FocusPanel { view: input_view });

    view.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(emitted.borrow().as_slice(), &[String::from("Ada!")]);

    Ok(())
}

/// Verifies focused text-area editing works inside component boundaries.
///
/// # Example Under Test
///
/// ```text
/// component(FocusPanel { view: text_area("Ada").on_input(...) })
/// Tab, A, Enter
/// ```
///
/// # Assertions
///
/// - Tabbing into the component boundary succeeds.
/// - The enter key is handled by the focused text area.
/// - The callback receives `Ada\n`.
#[test]
fn focused_text_area_inside_component_boundary_handles_editing_keys() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_text_area = Rc::clone(&emitted);
    let text_area_view = text_area("Ada").on_input(move |next| {
        emitted_for_text_area.borrow_mut().push(next);
        AppControl::Continue
    });
    let mut view = component(FocusPanel {
        view: text_area_view,
    });

    view.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(emitted.borrow().as_slice(), &[String::from("Ada\n")]);

    Ok(())
}

/// Verifies activation keys do not activate focused editable controls.
///
/// # Example Under Test
///
/// ```text
/// column([Input, button("Submit")])
/// Tab, Enter, Space, Tab, Enter
/// ```
///
/// # Assertions
///
/// - The editable input receives focus.
/// - Enter and space return [`AppControl::Continue`] without running callbacks
///   while the editable input is focused.
/// - The button receives focus after another tab event.
/// - Enter returns [`AppControl::Continue`] and runs the focused button callback.
#[test]
fn enter_and_space_do_not_activate_focused_editable_controls() -> Result<()> {
    let count = Rc::new(Cell::new(0));
    let submit_count = Rc::clone(&count);
    let mut view = column([
        editable_input("Ada"),
        button("Submit").on_press(move || {
            submit_count.set(submit_count.get() + 1);
            AppControl::Continue
        }),
    ]);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![true, false]);

    assert_eq!(
        view.handle_event(key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(
        view.handle_event(key(KeyCode::Char(' ')))?,
        AppControl::Continue
    );
    assert_eq!(count.get(), 0);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![false, true]);
    assert_eq!(
        view.handle_event(key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(count.get(), 1);

    Ok(())
}

/// Verifies focused button actions can request app exit.
///
/// # Example Under Test
///
/// ```text
/// button("Quit").on_press(|| AppControl::Exit)
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - The tab event succeeds and focuses the button.
/// - The enter event returns [`AppControl::Exit`].
#[test]
fn focused_button_action_can_exit_app_loop() -> Result<()> {
    let mut view = button("Quit").on_press(|| AppControl::Exit);

    view.handle_event(key(KeyCode::Tab))?;

    assert_eq!(view.handle_event(key(KeyCode::Enter))?, AppControl::Exit);

    Ok(())
}

/// Verifies focused buttons render with focus stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// row([button("One"), button("Two")])
/// Stylesheet::new().rule(StyleSelector::focus(), black on yellow)
/// with_focus(true)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rendered focused button label exists.
/// - The focused cell has a black foreground.
/// - The focused cell has a yellow background.
///
/// # Why
///
/// Focus selector state should affect rendered button styling.
#[test]
fn renders_focused_button_with_focus_stylesheet_rule() -> Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let view = row([button("One").with_focus(true), button("Two")]);
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::focus(),
        TuiStyle::new()
            .foreground(Color::Black)
            .background(Color::Yellow),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let focused_cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "O")
        .expect("rendered focused button label");

    assert_eq!(focused_cell.fg, Color::Black);
    assert_eq!(focused_cell.bg, Color::Yellow);

    Ok(())
}

/// Component that forwards rendering and built-in focus traversal to a child view.
struct FocusPanel {
    /// Child view owned by this component boundary.
    view: View,
}

impl Component for FocusPanel {
    /// Renders the child view.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the view boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        ctx.render_view(&self.view)
    }

    /// Returns the minimum useful render height of the child view.
    #[doc(hidden)]
    fn __min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        self.view.__min_height(ctx)
    }

    /// Returns the number of focusable controls inside the child view.
    #[doc(hidden)]
    fn __focusable_count(&self) -> usize {
        self.view.__focusable_count()
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.view.__focused_index_inner(index)
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        self.view.__set_focus_by_index_inner(target, index);
    }

    /// Returns the focused control's vertical span inside the child view.
    #[doc(hidden)]
    fn __focused_button_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        self.view.__focused_button_span(ctx)
    }

    /// Activates the focused control inside the child view, if any.
    #[doc(hidden)]
    fn __activate_focused_button(&self) -> Option<AppControl> {
        self.view.__activate_focused_button()
    }

    /// Handles keys for a focused input inside the child view, if any.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to apply to the focused child input.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the key control result when an input handles
    /// the key.
    #[doc(hidden)]
    fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.view.__handle_focused_input_key(key)
    }

    /// Returns the focused built-in control inside the child view.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing focused control metadata when a supported
    /// built-in control is focused.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<leptatui::__private::FocusedControl> {
        self.view.__focused_control()
    }

    /// Handles form-owned submit or cancel keys inside the child view.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to evaluate for nested form behavior.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing key traversal control when a nested form
    /// handles the key.
    #[doc(hidden)]
    fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.view.__handle_form_key(key)
    }
}

/// Component that renders text and exits on any event.
struct EventExit;

impl Component for EventExit {
    /// Renders the component's child text.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the view boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        ctx.render_view(&text("Child"))
    }

    /// Handles an event by requesting app exit.
    ///
    /// # Arguments
    ///
    /// * `_event` — Event dispatched through the view tree.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value requesting exit.
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        Ok(AppControl::Exit)
    }
}

/// Component that counts how many events it receives.
struct EventCounter {
    /// Shared event count updated by event handling.
    count: Rc<Cell<usize>>,
}

impl Component for EventCounter {
    /// Renders nothing for event-only tests.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Rendering context supplied by the view boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }

    /// Handles an event by incrementing the shared count.
    ///
    /// # Arguments
    ///
    /// * `_event` — Event dispatched through the view tree.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value requesting continued traversal.
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        self.count.set(self.count.get() + 1);
        Ok(AppControl::Continue)
    }
}

/// Component that records selector metadata from a rendered child view.
struct MetadataRecorder {
    /// Shared slot receiving child selector metadata.
    seen: Rc<RefCell<Option<StyleMetadata>>>,
}

impl Component for MetadataRecorder {
    /// Renders a child view and records its selector metadata.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the component boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let view = text("Child")
            .with_id("inside")
            .with_classes("component-child");
        *self.seen.borrow_mut() = view.style_metadata().cloned();
        ctx.render_view(&view)
    }
}

/// Verifies selector metadata remains available inside component boundaries.
///
/// # Example Under Test
///
/// ```text
/// component(MetadataRecorder)
/// text("Child").with_id("inside").with_classes("component-child")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component render call succeeds.
/// - Child metadata is recorded.
/// - The recorded id is `inside`.
/// - The recorded classes contain `component-child`.
///
/// # Why
///
/// Component boundaries should not prevent child views from carrying selector
/// metadata used by stylesheets.
#[test]
fn selector_metadata_remains_available_inside_component_boundaries() -> Result<()> {
    let seen = Rc::new(RefCell::new(None));
    let view = component(MetadataRecorder {
        seen: Rc::clone(&seen),
    });
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    let metadata = seen.borrow().clone().expect("recorded metadata");
    assert_eq!(metadata.id(), Some("inside"));
    assert_eq!(metadata.classes(), &[String::from("component-child")]);

    Ok(())
}

/// Verifies dynamic and component view boundaries render through the view tree.
///
/// # Example Under Test
///
/// ```text
/// column([dynamic(|| text("Dynamic")), component(EventExit)])
/// ```
///
/// # Assertions
///
/// - The dynamic closure is evaluated during rendering.
/// - The component boundary renders through its `Component::render` method.
#[test]
fn renders_dynamic_and_component_child_views() -> Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let view = column([dynamic(|| text("Dynamic")), component(EventExit)]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Dynamic"));
    assert!(rendered.contains("Child"));

    Ok(())
}

/// Verifies view roots dispatch events through component child boundaries.
///
/// # Example Under Test
///
/// ```text
/// column([text("Static"), component(EventExit)])
///     .handle_event(Event::Resize(24, 5))
/// ```
///
/// # Assertions
///
/// - Static leaf views are skipped.
/// - Event traversal reaches the component boundary.
/// - `AppControl::Exit` short-circuits child traversal.
#[test]
fn dispatches_events_through_component_child_views() -> Result<()> {
    let mut view = column([text("Static"), component(EventExit)]);

    assert_eq!(view.handle_event(Event::Resize(24, 5))?, AppControl::Exit);

    Ok(())
}

/// Verifies dynamic children are also traversed during event dispatch.
///
/// # Example Under Test
///
/// ```text
/// column([dynamic(|| component(EventCounter))])
///     .handle_event(Event::Resize(24, 5))
/// ```
///
/// # Assertions
///
/// - The dynamic closure is evaluated during event dispatch.
/// - Events reach the view produced by the dynamic closure.
#[test]
fn dispatches_events_through_dynamic_child_views() -> Result<()> {
    let count = Rc::new(Cell::new(0));
    let child_count = Rc::clone(&count);
    let mut view = column([dynamic(move || {
        component(EventCounter {
            count: Rc::clone(&child_count),
        })
    })]);

    assert_eq!(
        view.handle_event(Event::Resize(24, 5))?,
        AppControl::Continue
    );
    assert_eq!(count.get(), 1);

    Ok(())
}

/// Verifies deferred view equality stays identity-based.
///
/// # Example Under Test
///
/// ```text
/// let first = dynamic(|| text("same"));
/// let first_clone = first.clone();
/// let second = dynamic(|| text("same"));
/// ```
///
/// # Assertions
///
/// - A cloned dynamic view compares equal to its source.
/// - Separate dynamic views with identical closures do not compare equal.
#[test]
fn compares_dynamic_views_by_identity() {
    let first = dynamic(|| text("same"));
    let first_clone = first.clone();
    let second = dynamic(|| text("same"));

    assert_eq!(first, first_clone);
    assert_ne!(first, second);
}

/// Verifies editable control reconciliation retains shared runtime state.
///
/// # Example Under Test
///
/// ```text
/// reconcile(Input, previous Input with editable state)
/// reconcile(TextArea, previous TextArea with editable state)
/// ```
///
/// # Assertions
///
/// - Matching editable variants preserve focus.
/// - Matching editable variants preserve cursor, scroll, mode, selection, yank,
///   undo, and redo state.
#[test]
fn reconciliation_retains_editable_state_for_matching_controls() {
    let retained_input_state = editable_state_fixture();
    let mut previous_input = editable_input("old").with_focus(true);
    if let View::Input { editable_state, .. } = &mut previous_input {
        *editable_state = retained_input_state.clone();
    }
    let mut next_input = editable_input("new");

    leptatui::__private::__reconcile_view(&mut next_input, &previous_input);

    assert!(next_input.style_metadata().unwrap().is_focused());
    assert_eq!(editable_state(&next_input), &retained_input_state);

    let retained_text_area_state = editable_state_fixture();
    let mut previous_text_area = editable_text_area("old notes").with_focus(true);
    if let View::TextArea { editable_state, .. } = &mut previous_text_area {
        *editable_state = retained_text_area_state.clone();
    }
    let mut next_text_area = editable_text_area("new notes");

    leptatui::__private::__reconcile_view(&mut next_text_area, &previous_text_area);

    assert!(next_text_area.style_metadata().unwrap().is_focused());
    assert_eq!(editable_state(&next_text_area), &retained_text_area_state);
}

/// Verifies editable control reconciliation does not leak state across unrelated views.
///
/// # Example Under Test
///
/// ```text
/// reconcile(TextArea, previous Input with editable state)
/// reconcile(Input, previous TextArea with editable state)
/// reconcile(Button, previous Input with editable state)
/// ```
///
/// # Assertions
///
/// - Mismatched editable variants do not preserve focus.
/// - Mismatched editable variants keep their fresh editable state.
/// - Buttons do not inherit focus from previous editable controls.
#[test]
fn reconciliation_does_not_leak_editable_state_to_unrelated_views() {
    let retained_state = editable_state_fixture();
    let mut previous_input = editable_input("old").with_focus(true);
    if let View::Input { editable_state, .. } = &mut previous_input {
        *editable_state = retained_state.clone();
    }

    let mut next_text_area = editable_text_area("new notes");
    let fresh_text_area = editable_text_area("new notes");
    leptatui::__private::__reconcile_view(&mut next_text_area, &previous_input);

    assert!(!next_text_area.style_metadata().unwrap().is_focused());
    assert_eq!(
        editable_state(&next_text_area),
        editable_state(&fresh_text_area)
    );
    assert_ne!(editable_state(&next_text_area), &retained_state);

    let mut previous_text_area = editable_text_area("old notes").with_focus(true);
    if let View::TextArea { editable_state, .. } = &mut previous_text_area {
        *editable_state = retained_state.clone();
    }

    let mut next_input = editable_input("new");
    let fresh_input = editable_input("new");
    leptatui::__private::__reconcile_view(&mut next_input, &previous_text_area);

    assert!(!next_input.style_metadata().unwrap().is_focused());
    assert_eq!(editable_state(&next_input), editable_state(&fresh_input));
    assert_ne!(editable_state(&next_input), &retained_state);

    let mut next_button = button("Submit");
    leptatui::__private::__reconcile_view(&mut next_button, &previous_input);

    assert!(!next_button.style_metadata().unwrap().is_focused());
}

/// Verifies dynamic reconciliation replaces newly produced nested dynamic boundaries.
///
/// # Example Under Test
///
/// ```text
/// dynamic(|| dynamic(|| text(route_label)))
/// ```
///
/// # Assertions
///
/// - The first render shows the initial inner dynamic closure output.
/// - Updating the outer dynamic state replaces the previous inner dynamic closure.
#[test]
fn dynamic_reconciliation_replaces_new_nested_dynamic_boundaries() -> Result<()> {
    let label = Rc::new(Cell::new("Home"));
    let dynamic_label = Rc::clone(&label);
    let view = dynamic(move || {
        let current = dynamic_label.get();
        dynamic(move || text(current))
    });
    let mut terminal = Terminal::new(TestBackend::new(16, 1))?;

    draw_view(&mut terminal, &view)?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Home"), "rendered text: {rendered:?}");

    label.set("Counter");
    draw_view(&mut terminal, &view)?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Counter"), "rendered text: {rendered:?}");

    Ok(())
}
