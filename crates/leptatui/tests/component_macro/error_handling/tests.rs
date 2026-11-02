/// Returns the terminal-cell column where a substring begins.
///
/// # Arguments
///
/// * `line` — Rendered terminal row to inspect.
/// * `needle` — Substring whose starting column should be returned.
///
/// # Returns
///
/// An optional zero-based terminal-cell column.
fn rendered_column(line: &str, needle: &str) -> Option<usize> {
    line.find(needle)
        .map(|byte| line[..byte].chars().count())
}

/// Returns every terminal-cell column where a substring begins.
///
/// # Arguments
///
/// * `line` — Rendered terminal row to inspect.
/// * `needle` — Substring whose starting columns should be returned.
///
/// # Returns
///
/// A [`Vec`] containing each zero-based terminal-cell column.
fn rendered_columns(line: &str, needle: &str) -> Vec<usize> {
    line.match_indices(needle)
        .map(|(byte, _)| line[..byte].chars().count())
        .collect()
}

/// Asserts every cell in a rendered six-by-three action button has one style.
///
/// # Arguments
///
/// * `buffer` — Rendered terminal buffer containing the button.
/// * `terminal_width` — Width used to convert coordinates into buffer indices.
/// * `left` — Leftmost button-border column.
/// * `top` — Topmost button-border row.
/// * `foreground` — Expected label and border foreground color.
/// * `background` — Expected complete button background color.
fn assert_button_style(
    buffer: &ratatui::buffer::Buffer,
    terminal_width: usize,
    left: usize,
    top: usize,
    foreground: Color,
    background: Color,
) {
    for row in top..top + 3 {
        for column in left..left + 6 {
            let cell = &buffer.content()[row * terminal_width + column];
            assert_eq!(cell.fg, foreground, "cell: {cell:?}");
            assert_eq!(cell.bg, background, "cell: {cell:?}");
        }
    }
}

/// Verifies a replacement view error displays only its custom message.
///
/// # Example Under Test
///
/// ```text
/// view_error!("custom display message")
/// source = "hidden source"
/// ```
///
/// # Assertions
///
/// - The isolated `Error` heading is rendered.
/// - File and line metadata identify the fallible component declaration.
/// - Blank rows separate the heading, metadata, and custom message.
/// - The custom message is rendered.
/// - The discarded source message is not rendered.
/// - A root error without router history renders Quit but not Back.
/// - Diagnostic and frame cells use red with no background.
/// - The initially focused Quit button uses white on a red background.
/// - Hostile component styles do not alter the frame or layout.
/// - The single Quit action is centered at the bottom of the screen.
#[test]
fn fallible_component_replaces_error_with_custom_message() -> leptatui::app::Result<()> {
    let mut component = MacroReplacedError::new();
    let terminal = render_component(&mut component, 80, 16)?;
    let text = rendered_text(&terminal);
    let lines = rendered_lines(&terminal);
    let buffer = terminal.backend().buffer();
    let cell = |column: usize, row: usize| &buffer.content()[row * 80 + column];
    let fixture_line = include_str!("fixtures.rs")
        .lines()
        .position(|line| line.contains("fn MacroReplacedError()"))
        .expect("replacement fixture declaration should remain present")
        + 1;

    assert!(text.contains("# Error"), "rendered text: {text:?}");
    assert!(
        text.contains("File: crates/leptatui/tests/component_macro/error_handling/fixtures.rs"),
        "rendered text: {text:?}",
    );
    assert!(
        text.contains(&format!("Line Number: {fixture_line}")),
        "rendered text: {text:?}",
    );
    assert!(text.contains("custom display message"), "rendered text: {text:?}");
    assert!(!text.contains("hidden source"), "rendered text: {text:?}");
    assert!(text.contains("Quit"), "rendered text: {text:?}");
    assert!(!text.contains("Back"), "rendered text: {text:?}");
    assert!(!text.contains("q: quit"), "rendered text: {text:?}");
    assert_eq!(cell(0, 0).symbol(), "╭");
    assert_eq!(cell(79, 0).symbol(), "╮");
    assert_eq!(cell(0, 15).symbol(), "╰");
    assert_eq!(cell(79, 15).symbol(), "╯");
    let quit_left = rendered_column(&lines[11], "┌────┐")
        .expect("focused Quit button should retain its top border");
    assert_button_style(buffer, 80, quit_left, 11, Color::White, Color::Red);
    for (index, cell) in buffer.content().iter().enumerate() {
        let column = index % 80;
        let row = index / 80;
        if (11..14).contains(&row) && (quit_left..quit_left + 6).contains(&column) {
            continue;
        }
        assert_eq!(cell.bg, Color::Reset, "cell: {cell:?}");
        if cell.symbol() != " " {
            assert_eq!(cell.fg, Color::Red, "cell: {cell:?}");
        }
    }
    let heading_row = lines.iter().position(|line| line.contains("# Error")).unwrap();
    let file_row = lines.iter().position(|line| line.contains("File:")).unwrap();
    let line_row = lines
        .iter()
        .position(|line| line.contains("Line Number:"))
        .unwrap();
    let message_row = lines
        .iter()
        .position(|line| line.contains("custom display message"))
        .unwrap();
    assert_eq!(file_row, heading_row + 2);
    assert_eq!(line_row, file_row + 2);
    assert_eq!(message_row, line_row + 2);
    assert_eq!(rendered_column(&lines[12], "Quit"), Some(38));

    Ok(())
}

/// Verifies a long diagnostic keeps recovery controls inside a small viewport.
///
/// # Example Under Test
///
/// ```text
/// viewport = 40x20
/// message = a diagnostic wider than the viewport
/// ```
///
/// # Assertions
///
/// - The diagnostic begins rendering beneath the heading.
/// - The Quit action remains visible on the bottom action row.
/// - The outer frame retains its bottom edge.
#[test]
fn fallible_component_long_error_keeps_bottom_controls_visible() -> leptatui::app::Result<()> {
    let mut component = MacroLongError::new();
    let terminal = render_component(&mut component, 40, 20)?;
    let lines = rendered_lines(&terminal);

    assert!(
        lines.iter().any(|line| line.contains("This intentionally long")),
        "rendered lines: {lines:?}",
    );
    assert_eq!(rendered_column(&lines[16], "Quit"), Some(18));
    assert_eq!(lines[19].chars().next(), Some('╰'));
    assert_eq!(lines[19].chars().last(), Some('╯'));

    Ok(())
}

/// Verifies a routed error clears its complete surrounding application shell.
///
/// # Example Under Test
///
/// ```text
/// bordered shell
///   title + navigation + healthy route + footer
/// e -> failing route
/// ```
///
/// # Assertions
///
/// - The healthy shell renders before navigation to the failure.
/// - The error screen retains its own diagnostic and controls.
/// - Shell title, navigation, route content, footer, and color are absent.
#[test]
fn fallible_component_error_replaces_complete_router_shell() -> leptatui::app::Result<()> {
    let mut component = MacroShellErrorRoot::new();
    let terminal = render_component(&mut component, 80, 18)?;
    let healthy = rendered_text(&terminal);
    assert!(healthy.contains("Leptatui error handling"));
    assert!(healthy.contains("Propagated error"));
    assert!(healthy.contains("Healthy page"));
    assert!(healthy.contains("h home | e propagated | q quit"));

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('e')))?,
        AppControl::Continue,
    );
    let terminal = render_component(&mut component, 80, 18)?;
    let error = rendered_text(&terminal);
    assert!(error.contains("routed failure"), "rendered text: {error:?}");
    assert!(error.contains("Back"), "rendered text: {error:?}");
    assert!(error.contains("Quit"), "rendered text: {error:?}");
    for hidden in [
        "Leptatui error handling",
        "Error route navigation",
        "Home",
        "Propagated error",
        "Healthy page",
        "h home | e propagated | q quit",
    ] {
        assert!(!error.contains(hidden), "unexpected shell text: {hidden:?}");
    }
    let top_left = &terminal.backend().buffer().content()[0];
    assert_eq!(top_left.symbol(), "╭");
    assert_eq!(top_left.fg, Color::Red);

    Ok(())
}

/// Verifies a contextual view error retains its source chain.
///
/// # Example Under Test
///
/// ```text
/// view_error!(error => "custom context")
/// error = "source detail"
/// ```
///
/// # Assertions
///
/// - The custom context is rendered.
/// - The original source detail is rendered after the context.
#[test]
fn fallible_component_preserves_context_source_chain() -> leptatui::app::Result<()> {
    let mut component = MacroContextError::new();
    let terminal = render_component(&mut component, 80, 16)?;
    let text = rendered_text(&terminal);

    assert!(text.contains("custom context"), "rendered text: {text:?}");
    assert!(text.contains("source detail"), "rendered text: {text:?}");

    Ok(())
}

/// Verifies error-screen Quit and router Back keyboard controls.
///
/// # Example Under Test
///
/// ```text
/// Healthy page -> e -> routed failure -> b -> Healthy page
/// root failure -> q -> AppControl::Exit
/// ```
///
/// # Assertions
///
/// - `q` exits from an error screen without router history.
/// - Enter and Space activate the initially focused Quit control.
/// - A routed error renders Back and the failure message.
/// - Back and Quit have evenly sized outer and intervening gaps.
/// - Back begins focused with white-on-red styling while Quit remains red.
/// - Tab moves focus and white-on-red styling from Back to Quit.
/// - Ancestor page styles do not alter the routed error screen.
/// - `b` and Escape return to the prior healthy route.
#[test]
fn fallible_component_error_controls_quit_and_go_back() -> leptatui::app::Result<()> {
    let mut root_error = MacroReplacedError::new();
    assert_eq!(
        View::handle_event(&mut root_error, key(KeyCode::Char('q')))?,
        AppControl::Exit,
    );
    let mut enter_error = MacroReplacedError::new();
    assert_eq!(
        View::handle_event(&mut enter_error, key(KeyCode::Enter))?,
        AppControl::Exit,
    );
    let mut space_error = MacroReplacedError::new();
    assert_eq!(
        View::handle_event(&mut space_error, key(KeyCode::Char(' ')))?,
        AppControl::Exit,
    );

    let mut routed = MacroErrorRouteRoot::new();
    let terminal = render_component(&mut routed, 80, 18)?;
    assert!(rendered_text(&terminal).contains("Healthy page"));

    assert_eq!(
        View::handle_event(&mut routed, key(KeyCode::Char('e')))?,
        AppControl::Continue,
    );
    let terminal = render_component(&mut routed, 80, 18)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("routed failure"), "rendered text: {text:?}");
    assert!(text.contains("Back"), "rendered text: {text:?}");
    let lines = rendered_lines(&terminal);
    let action_border = &lines[13];
    let button_starts = rendered_columns(action_border, "┌────┐");
    assert_eq!(button_starts.len(), 2, "action row: {action_border:?}");
    let buffer = terminal.backend().buffer();
    assert_button_style(
        buffer,
        80,
        button_starts[0],
        13,
        Color::White,
        Color::Red,
    );
    assert_button_style(
        buffer,
        80,
        button_starts[1],
        13,
        Color::Red,
        Color::Reset,
    );
    let gaps = [
        button_starts[0].saturating_sub(2),
        button_starts[1].saturating_sub(button_starts[0] + 6),
        78usize.saturating_sub(button_starts[1] + 6),
    ];
    assert!(
        gaps.iter().max().unwrap() - gaps.iter().min().unwrap() <= 1,
        "action gaps: {gaps:?}",
    );

    assert_eq!(
        View::handle_event(&mut routed, key(KeyCode::Tab))?,
        AppControl::Continue,
    );
    let terminal = render_component(&mut routed, 80, 18)?;
    let buffer = terminal.backend().buffer();
    assert_button_style(
        buffer,
        80,
        button_starts[0],
        13,
        Color::Red,
        Color::Reset,
    );
    assert_button_style(
        buffer,
        80,
        button_starts[1],
        13,
        Color::White,
        Color::Red,
    );

    assert_eq!(
        View::handle_event(&mut routed, key(KeyCode::Char('b')))?,
        AppControl::Continue,
    );
    let terminal = render_component(&mut routed, 80, 18)?;
    assert!(rendered_text(&terminal).contains("Healthy page"));

    assert_eq!(
        View::handle_event(&mut routed, key(KeyCode::Char('e')))?,
        AppControl::Continue,
    );
    render_component(&mut routed, 80, 18)?;
    assert_eq!(
        View::handle_event(&mut routed, key(KeyCode::Esc))?,
        AppControl::Continue,
    );
    let terminal = render_component(&mut routed, 80, 18)?;
    assert!(rendered_text(&terminal).contains("Healthy page"));

    Ok(())
}
