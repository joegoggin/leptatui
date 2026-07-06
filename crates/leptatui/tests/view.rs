//! View rendering tests.
//!
//! These tests render view trees against Ratatui's test backend and inspect the
//! resulting terminal buffer.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use leptatui::{
    AppControl, Color, Component, EditableState, KeyControl, LayoutDirection, MediaQuery,
    MiniVimMode, RenderCtx, Result, StyleMetadata, StyleSelector, Stylesheet, TuiStyle, View,
    ViewType, block, button, column, component, dynamic, input, row, text, text_area,
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    symbols::{block as symbol_block, line as symbol_line},
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
        View::Row { children, .. } | View::Column { children, .. } => {
            children.iter().flat_map(button_focuses).collect()
        }
        View::Text { .. }
        | View::Input { .. }
        | View::TextArea { .. }
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
        View::Row { children, .. } | View::Column { children, .. } => {
            children.iter().flat_map(control_focuses).collect()
        }
        View::Text { .. } | View::Dynamic(_) | View::Component(_) => Vec::new(),
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
/// An [`EditableState`] value containing cursor, scroll, mode, yank, undo, and
/// redo state.
fn editable_state_fixture() -> EditableState {
    let mut state = EditableState::new();
    state.set_cursor(6);
    state.set_horizontal_scroll(2);
    state.set_vertical_scroll(3);
    state.set_mode(MiniVimMode::Normal);
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

/// Creates a focused input view that records emitted values.
///
/// # Arguments
///
/// * `value` — Initial controlled input value.
/// * `emitted` — Shared vector that receives callback values.
///
/// # Returns
///
/// A focused [`View`] configured as an input.
fn emitting_input(value: impl Into<String>, emitted: &Rc<RefCell<Vec<String>>>) -> View {
    let emitted_for_input = Rc::clone(emitted);
    input(value).with_focus(true).on_input(move |next| {
        emitted_for_input.borrow_mut().push(next);
        AppControl::Continue
    })
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
/// A focused [`View`] configured as a text area.
fn emitting_text_area(value: impl Into<String>, emitted: &Rc<RefCell<Vec<String>>>) -> View {
    let emitted_for_text_area = Rc::clone(emitted);
    text_area(value).with_focus(true).on_input(move |next| {
        emitted_for_text_area.borrow_mut().push(next);
        AppControl::Continue
    })
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
/// reused.
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

fn scroll_offset(view: &View) -> u16 {
    match view {
        View::Row { metadata, .. } | View::Column { metadata, .. } => metadata.scroll_offset(),
        other => panic!("expected layout view, got {other:?}"),
    }
}

fn symbol_position(terminal: &Terminal<TestBackend>, symbol: &str, width: u16) -> (u16, u16) {
    symbol_position_opt(terminal, symbol, width)
        .unwrap_or_else(|| panic!("rendered `{symbol}` cell"))
}

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

fn cell_symbol(terminal: &Terminal<TestBackend>, x: u16, y: u16, width: u16) -> &str {
    let index = usize::from(y) * usize::from(width) + usize::from(x);
    terminal.backend().buffer().content()[index].symbol()
}

fn cell_colors(terminal: &Terminal<TestBackend>, x: u16, y: u16, width: u16) -> (Color, Color) {
    let index = usize::from(y) * usize::from(width) + usize::from(x);
    let cell = &terminal.backend().buffer().content()[index];
    (cell.fg, cell.bg)
}

fn draw_view(terminal: &mut Terminal<TestBackend>, view: &View) -> Result<()> {
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;

    render_result
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
/// width = 8
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The first three rendered cells contain `A`, `d`, and `a`.
#[test]
fn renders_input_value() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = input("Ada");

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "A");
    assert_eq!(cell_symbol(&terminal, 1, 0, 8), "d");
    assert_eq!(cell_symbol(&terminal, 2, 0, 8), "a");

    Ok(())
}

/// Verifies empty input views render placeholder text.
///
/// # Example Under Test
///
/// ```text
/// input("").placeholder("Name")
/// width = 8
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The rendered cells contain the first and last placeholder characters.
#[test]
fn renders_input_placeholder_when_value_is_empty() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = input("").placeholder("Name");

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "N");
    assert_eq!(cell_symbol(&terminal, 3, 0, 8), "e");

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
    let backend = TestBackend::new(8, 1);
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

    let (fg, bg) = cell_colors(&terminal, 0, 0, 8);
    assert_eq!(fg, Color::Black);
    assert_eq!(bg, Color::Yellow);

    Ok(())
}

/// Verifies input rendering clips content around the retained cursor.
///
/// # Example Under Test
///
/// ```text
/// input("abcdef").with_focus(true)
/// width = 4
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
    let backend = TestBackend::new(4, 1);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("abcdef").with_focus(true);

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 0, 0, 4), "c");
    assert_eq!(cell_symbol(&terminal, 3, 0, 4), "f");

    editable_state_mut(&mut view).set_cursor(0);
    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 0, 0, 4), "a");
    assert_eq!(cell_symbol(&terminal, 3, 0, 4), "d");

    Ok(())
}

/// Verifies text-area views render multiline controlled values.
///
/// # Example Under Test
///
/// ```text
/// text_area("One\nTwo")
/// width = 8, height = 2
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The first line starts on the first terminal row.
/// - The second line starts on the second terminal row.
#[test]
fn renders_text_area_multiline_value() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("One\nTwo");

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
/// width = 8
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The rendered cells contain the first and last placeholder characters.
#[test]
fn renders_text_area_placeholder_when_value_is_empty() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("").placeholder("Notes");

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "N");
    assert_eq!(cell_symbol(&terminal, 4, 0, 8), "s");

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
    let backend = TestBackend::new(8, 2);
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

    let (fg, bg) = cell_colors(&terminal, 0, 0, 8);
    assert_eq!(fg, Color::Black);
    assert_eq!(bg, Color::Yellow);

    Ok(())
}

/// Verifies text-area rendering scrolls vertically around the retained cursor.
///
/// # Example Under Test
///
/// ```text
/// text_area("aaa\nbbb\nccc").with_focus(true)
/// height = 2
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
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = text_area("aaa\nbbb\nccc").with_focus(true);

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "b");
    assert_eq!(cell_symbol(&terminal, 0, 1, 8), "c");

    editable_state_mut(&mut view).set_cursor(0);
    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "a");
    assert_eq!(cell_symbol(&terminal, 0, 1, 8), "b");

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
    let backend = TestBackend::new(6, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = column(vec![text_area("Hello World"), text("End")]);

    draw_view(&mut terminal, &view)?;

    assert_eq!(symbol_position(&terminal, "E", 6), (0, 2));

    Ok(())
}

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

    assert_eq!(
        char_view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_space = Rc::clone(&emitted);
    let mut space_view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_space.borrow_mut().push(next);
        AppControl::Continue
    });

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

    assert_eq!(
        backspace_view.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_delete = Rc::clone(&emitted);
    let mut delete_view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_delete.borrow_mut().push(next);
        AppControl::Continue
    });
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
    let backend = TestBackend::new(8, 1);
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
    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "A");
    assert_eq!(cell_symbol(&terminal, 2, 0, 8), "a");
    assert_eq!(cell_symbol(&terminal, 3, 0, 8), " ");

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

    assert_eq!(
        char_view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_enter = Rc::clone(&emitted);
    let mut enter_view = text_area("Ada").with_focus(true).on_input(move |next| {
        emitted_for_enter.borrow_mut().push(next);
        AppControl::Continue
    });

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

/// Verifies editable controls support mini-Vim mode transition keys.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// Esc, i, a, I, A
///
/// text_area("ab\ncd").with_focus(true)
/// I, A
/// ```
///
/// # Assertions
///
/// - Inputs start in insert mode.
/// - Esc switches the input to normal mode and moves the cursor onto the
/// previous character.
/// - `i` and `a` switch the input to insert mode at the current and next
/// normal-mode positions.
/// - `I` and `A` move to the line start and line end for inputs and text
/// areas.
#[test]
fn focused_editable_controls_support_mini_vim_mode_transitions() -> Result<()> {
    let mut input_view = input("Ada").with_focus(true);
    assert_eq!(editable_state(&input_view).mode(), MiniVimMode::Insert);

    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&input_view).mode(), MiniVimMode::Normal);
    assert_eq!(editable_state(&input_view).cursor(), 2);

    editable_state_mut(&mut input_view).set_cursor(1);
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Char('i')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&input_view).mode(), MiniVimMode::Insert);
    assert_eq!(editable_state(&input_view).cursor(), 1);

    editable_state_mut(&mut input_view).set_mode(MiniVimMode::Normal);
    editable_state_mut(&mut input_view).set_cursor(1);
    input_view.handle_key_event(key_event(KeyCode::Char('a')))?;
    assert_eq!(editable_state(&input_view).mode(), MiniVimMode::Insert);
    assert_eq!(editable_state(&input_view).cursor(), 2);

    editable_state_mut(&mut input_view).set_mode(MiniVimMode::Normal);
    editable_state_mut(&mut input_view).set_cursor(2);
    input_view.handle_key_event(key_event(KeyCode::Char('I')))?;
    assert_eq!(editable_state(&input_view).cursor(), 0);

    editable_state_mut(&mut input_view).set_mode(MiniVimMode::Normal);
    editable_state_mut(&mut input_view).set_cursor(0);
    input_view.handle_key_event(key_event(KeyCode::Char('A')))?;
    assert_eq!(editable_state(&input_view).cursor(), 3);

    let mut text_area_view = text_area("ab\ncd").with_focus(true);
    editable_state_mut(&mut text_area_view).set_mode(MiniVimMode::Normal);
    editable_state_mut(&mut text_area_view).set_cursor(4);
    text_area_view.handle_key_event(key_event(KeyCode::Char('I')))?;
    assert_eq!(editable_state(&text_area_view).mode(), MiniVimMode::Insert);
    assert_eq!(editable_state(&text_area_view).cursor(), 3);

    editable_state_mut(&mut text_area_view).set_mode(MiniVimMode::Normal);
    editable_state_mut(&mut text_area_view).set_cursor(4);
    text_area_view.handle_key_event(key_event(KeyCode::Char('A')))?;
    assert_eq!(editable_state(&text_area_view).cursor(), 5);

    Ok(())
}

/// Verifies focused inputs support mini-Vim normal-mode movement.
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
fn focused_input_supports_mini_vim_normal_mode_movement() -> Result<()> {
    let mut view = input("one two three").with_focus(true);
    editable_state_mut(&mut view).set_mode(MiniVimMode::Normal);
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

/// Verifies focused text areas support mini-Vim normal-mode movement.
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
fn focused_text_area_supports_mini_vim_normal_mode_movement() -> Result<()> {
    let mut view = text_area("one\ntwo\nthree").with_focus(true);
    editable_state_mut(&mut view).set_mode(MiniVimMode::Normal);
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
fn focused_input_supports_mini_vim_delete_yank_paste_undo_and_redo() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("abc", &emitted);
    editable_state_mut(&mut view).set_mode(MiniVimMode::Normal);
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
/// buffer.
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
    editable_state_mut(&mut view).set_mode(MiniVimMode::Normal);
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
    let backend = TestBackend::new(12, 2);
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
    assert_eq!(cell_symbol(&terminal, 0, 0, 12), "A");
    assert_eq!(cell_symbol(&terminal, 2, 0, 12), "a");
    assert_eq!(cell_symbol(&terminal, 3, 0, 12), " ");

    Ok(())
}

/// Verifies focused input editing works inside component boundaries.
///
/// # Example Under Test
///
/// ```text
/// component(FocusPanel { view: input("Ada").on_input(...) })
/// Tab, Char('!')
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
/// Tab, Enter
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
/// - Matching editable variants preserve cursor, scroll, mode, yank, undo, and
///   redo state.
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
