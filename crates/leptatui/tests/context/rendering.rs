/// Verifies component render scopes can provide and read typed context.
///
/// # Example Under Test
///
/// ```text
/// AppRoot::render(&mut ContextRoot, frame)
/// provide_context(String::from("from component"))
/// leptos::context::provide_context(ReadSignal<i32>)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The app root render call succeeds.
/// - The component reads the string context from the Leptatui render scope.
/// - The component reads the signal value from Leptos owner context fallback.
///
/// # Why
///
/// Component rendering bridges Leptatui render scopes and Leptos owner scopes.
#[test]
fn component_render_scope_can_provide_and_read_context() -> Result<()> {
    let backend = TestBackend::new(16, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut root = ContextRoot {
        observed_text: RefCell::new(None),
        observed_count: RefCell::new(None),
    };
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        render_result = AppRoot::render(&mut root, frame);
    })?;
    render_result?;

    assert_eq!(
        root.observed_text.borrow().as_deref(),
        Some("from component")
    );
    assert_eq!(*root.observed_count.borrow(), Some(2));

    Ok(())
}

/// Verifies direct theme variable context updates rendered stylesheet colors.
///
/// # Example Under Test
///
/// A `ThemeRenderRoot` provides light colors when `dark` is false and dark
/// colors when `dark` is true.
///
/// # Assertions
///
/// - The first render succeeds and paints the themed text black on white.
/// - Updating the signal to dark mode succeeds.
/// - The second render succeeds and paints the themed text white on black.
#[test]
fn context_theme_variables_update_rendered_styles() -> Result<()> {
    let owner = Owner::new();
    let (dark, set_dark) = owner.with(|| signal(false));
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("themed"),
        StyleDeclarations::new()
            .foreground(theme_color("text"))
            .background(theme_color("surface")),
    );
    let mut root = ThemeRenderRoot {
        dark,
        child: text("Theme").with_classes("themed").into_view(),
        stylesheet,
    };
    let backend = TestBackend::new(12, 1);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        render_result = AppRoot::render(&mut root, frame);
    })?;
    render_result?;
    let cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "T")
        .expect("rendered themed cell");
    assert_eq!(cell.fg, Color::Black);
    assert_eq!(cell.bg, Color::White);

    set_dark.set(true);

    render_result = Ok(());
    terminal.draw(|frame| {
        render_result = AppRoot::render(&mut root, frame);
    })?;
    render_result?;
    let cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "T")
        .expect("rendered themed cell");
    assert_eq!(cell.fg, Color::White);
    assert_eq!(cell.bg, Color::Black);

    Ok(())
}

/// Verifies theme signal context updates rendered stylesheet colors.
///
/// # Example Under Test
///
/// A `ThemeSignalRoot` provides a `ReadSignal<ThemeVariables>` consumed by
/// theme-backed stylesheet declarations.
///
/// # Assertions
///
/// - The first render succeeds and resolves light theme colors.
/// - Updating the theme signal succeeds.
/// - The second render succeeds and resolves dark theme colors.
#[test]
fn context_theme_signal_updates_rendered_styles() -> Result<()> {
    let owner = Owner::new();
    let light = ThemeVariables::new()
        .color("text", Color::Black)
        .color("surface", Color::White);
    let dark = ThemeVariables::new()
        .color("text", Color::White)
        .color("surface", Color::Black);
    let (theme, set_theme) = owner.with(|| signal(light));
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("themed"),
        StyleDeclarations::new()
            .foreground(theme_color("text"))
            .background(theme_color("surface")),
    );
    let mut root = ThemeSignalRoot {
        theme,
        child: text("Theme").with_classes("themed").into_view(),
        stylesheet,
    };
    let backend = TestBackend::new(12, 1);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        render_result = AppRoot::render(&mut root, frame);
    })?;
    render_result?;
    let cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "T")
        .expect("rendered themed cell");
    assert_eq!(cell.fg, Color::Black);
    assert_eq!(cell.bg, Color::White);

    set_theme.set(dark);

    render_result = Ok(());
    terminal.draw(|frame| {
        render_result = AppRoot::render(&mut root, frame);
    })?;
    render_result?;
    let cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "T")
        .expect("rendered themed cell");
    assert_eq!(cell.fg, Color::White);
    assert_eq!(cell.bg, Color::Black);

    Ok(())
}
