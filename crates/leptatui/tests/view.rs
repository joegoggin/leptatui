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
    AppControl, Color, Component, KeyControl, LayoutDirection, MediaQuery, RenderCtx, Result,
    StyleMetadata, StyleSelector, Stylesheet, TuiStyle, View, ViewType, block, button, column,
    component, dynamic, row, text,
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
        View::Text { .. } | View::Dynamic(_) | View::Component(_) => Vec::new(),
    }
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
