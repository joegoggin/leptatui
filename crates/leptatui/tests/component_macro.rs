//! Compile tests for Leptatui component macros.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use leptatui::context::provide_context;
use leptatui::{
    AppControl, Color, Component, KeyControl, RenderCtx, Result, ThemeVariables, button, column,
    component, dynamic, row, stylesheet, text, theme_color, use_key_event,
};
use leptos::prelude::{GetUntracked, Update, signal};
use ratatui::{Terminal, backend::TestBackend};

static MACRO_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_SIGNAL_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
static MACRO_CONTEXT_OBSERVED: Mutex<Option<MacroLabel>> = Mutex::new(None);
static MACRO_PARENT_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_CHILD_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_PASS_PARENT_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_PASS_CHILD_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_FIRST_KEY_HANDLER: AtomicUsize = AtomicUsize::new(0);
static MACRO_SECOND_KEY_HANDLER: AtomicUsize = AtomicUsize::new(0);
static MACRO_THIRD_KEY_HANDLER: AtomicUsize = AtomicUsize::new(0);
static MACRO_DEFAULT_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_REPEAT_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_RELEASE_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);

/// Context value used by generated component provider tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MacroLabel(&'static str);

/// Component with a local stylesheet applied to its own text node.
#[component]
fn MacroStyledText() -> leptatui::Node {
    stylesheet! {
        .scoped => { fg: Color::Yellow, bg: Color::Blue }
    }

    text("Scoped").with_classes("scoped")
}

/// Component whose stylesheet targets a shared class name.
#[component]
fn MacroStyledSibling() -> leptatui::Node {
    stylesheet! {
        .shared => { fg: Color::Yellow }
    }

    text("Styled").with_classes("shared")
}

/// Component with a class that should not receive sibling styles.
#[component]
fn MacroPlainSibling() -> leptatui::Node {
    text("Plain").with_classes("shared")
}

/// Parent component whose stylesheet should apply to child component internals.
#[component]
fn MacroParentStylesChild() -> leptatui::Node {
    stylesheet! {
        Text => { fg: Color::Green }
    }

    component(MacroPlainSibling::new())
}

/// Parent and child components that both style text.
#[component]
fn MacroParentWithChildOverride() -> leptatui::Node {
    stylesheet! {
        Text => { fg: Color::Green }
    }

    component(MacroChildStyleOverride::new())
}

/// Child component whose stylesheet should override parent component styles.
#[component]
fn MacroChildStyleOverride() -> leptatui::Node {
    stylesheet! {
        Text => { fg: Color::Yellow }
    }

    text("Override")
}

/// Component whose stylesheet resolves against theme context it provides.
#[component]
fn MacroThemedStylesheet() -> leptatui::Node {
    provide_context(ThemeVariables::new().color("text", Color::LightCyan));

    stylesheet! {
        .themed => { fg: theme_color("text") }
    }

    text("Theme").with_classes("themed")
}

/// Parent component with styled and plain sibling component subtrees.
#[component]
fn MacroSiblingStyleRoot() -> leptatui::Node {
    row([
        component(MacroStyledSibling::new()),
        component(MacroPlainSibling::new()),
    ])
}

/// Component with an interactive button used by macro runtime tests.
#[component]
fn MacroButtonRoot() -> leptatui::Node {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('s') {
            MACRO_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    button("Save").on_press(|| {
        MACRO_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}

/// Component with no matching hook for default button key tests.
#[component]
fn MacroDefaultButtonRoot() -> leptatui::Node {
    button("Default").on_press(|| {
        MACRO_DEFAULT_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}

/// Component whose key map handles Tab before focus can move.
#[component]
fn MacroTabOverrideButtonRoot() -> leptatui::Node {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Tab {
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    button("Default").on_press(|| {
        MACRO_DEFAULT_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}

/// Component whose key map handles Enter before a focused button activates.
#[component]
fn MacroEnterOverrideButtonRoot() -> leptatui::Node {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Enter {
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    button("Default").on_press(|| {
        MACRO_DEFAULT_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}

/// Component with local signal state created during generated setup.
#[component]
fn MacroSignalRoot() -> leptatui::Node {
    MACRO_SIGNAL_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);
    let (count, set_count) = signal(0);
    let increment = set_count;

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('i') {
            increment.update(|count| *count += 1);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    column([
        dynamic(move || text(format!("Count: {}", count.get_untracked()))),
        button("Increment").on_press(move || {
            set_count.update(|count| *count += 1);
            AppControl::Continue
        }),
    ])
}

/// Component that records the label visible from its render context.
struct MacroContextConsumer;

impl Component for MacroContextConsumer {
    /// Records the context label visible during render.
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        *MACRO_CONTEXT_OBSERVED
            .lock()
            .expect("context observation lock should be available") =
            leptatui::context::use_context::<MacroLabel>();
        Ok(())
    }
}

/// Component that provides context to a descendant component boundary.
#[component]
fn MacroContextProvider() -> leptatui::Node {
    leptatui::context::provide_context(MacroLabel("macro"));
    component(MacroContextConsumer)
}

/// Component that exits when `q` is pressed.
#[component]
fn MacroKeyExitRoot() -> leptatui::Node {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    text("Press q")
}

/// Parent key map used to prove child handlers get priority.
#[component]
fn MacroParentKeyRoot() -> leptatui::Node {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('x') {
            MACRO_PARENT_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    component(MacroChildKeyHandler::new())
}

/// Child key map that handles the same key as its parent.
#[component]
fn MacroChildKeyHandler() -> leptatui::Node {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('x') {
            MACRO_CHILD_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    text("Child")
}

/// Parent key map used to prove child pass-through reaches ancestors.
#[component]
fn MacroParentAfterPassRoot() -> leptatui::Node {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('p') {
            MACRO_PASS_PARENT_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    component(MacroPassingChildKeyHandler::new())
}

/// Child key map that observes a key but passes it to its parent.
#[component]
fn MacroPassingChildKeyHandler() -> leptatui::Node {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('p') {
            MACRO_PASS_CHILD_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
        }

        KeyControl::Pass
    });

    text("Child")
}

/// Component with several handlers used to prove source-order short-circuiting.
#[component]
fn MacroMultipleKeyHandlers() -> leptatui::Node {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('m') {
            MACRO_FIRST_KEY_HANDLER.fetch_add(1, Ordering::SeqCst);
        }

        KeyControl::Pass
    });
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('m') {
            MACRO_SECOND_KEY_HANDLER.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('m') {
            MACRO_THIRD_KEY_HANDLER.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    text("Handlers")
}

/// Component with explicit repeat and release key handlers.
#[component]
fn MacroKindSpecificKeyHandlers() -> leptatui::Node {
    use_key_event(KeyEventKind::Repeat, |key| {
        if key.code == KeyCode::Char('k') {
            MACRO_REPEAT_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });
    use_key_event(KeyEventKind::Release, |key| {
        if key.code == KeyCode::Char('k') {
            MACRO_RELEASE_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    text("Kinds")
}

/// Creates a key-press event for a key code.
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Creates a key event for a key code and kind.
fn key_with_kind(code: KeyCode, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind))
}

/// Returns the terminal buffer content as a flat string.
fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

/// Returns the foreground and background colors for the first matching symbol.
fn rendered_cell_colors(terminal: &Terminal<TestBackend>, symbol: &str) -> (Color, Color) {
    let cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == symbol)
        .unwrap_or_else(|| panic!("rendered `{symbol}` cell"));

    (cell.fg, cell.bg)
}

/// Renders a component into a test backend.
fn render_component<C>(component: &mut C, width: u16, height: u16) -> Result<Terminal<TestBackend>>
where
    C: Component,
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = Component::render(component, &mut ctx);
    })?;
    render_result?;

    Ok(terminal)
}

/// Verifies component macro pass and fail fixtures compile as expected.
///
/// # Example Under Test
///
/// ```text
/// tests/fixtures/component_macro/pass/*.rs
/// tests/fixtures/component_macro/fail/*.rs
/// ```
///
/// # Assertions
///
/// - Every pass fixture compiles successfully.
/// - Every fail fixture emits the expected compile error.
///
/// # Why
///
/// Component macro validation should reject unsupported signatures while
/// preserving accepted component conversions.
#[test]
fn component_macro_compile_cases() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/fixtures/component_macro/pass/*.rs");
    cases.compile_fail("tests/fixtures/component_macro/fail/*.rs");
}

/// Verifies a bare `stylesheet!` statement registers with its component.
///
/// # Assertions
///
/// - The component's text receives the foreground and background from the
///   local stylesheet.
#[test]
fn generated_component_stylesheet_styles_own_nodes() -> Result<()> {
    let mut component = MacroStyledText::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(
        rendered_cell_colors(&terminal, "S"),
        (Color::Yellow, Color::Blue)
    );

    Ok(())
}

/// Verifies component styles do not leak into sibling component subtrees.
///
/// # Assertions
///
/// - The styled sibling receives the shared class style.
/// - The plain sibling has the same class but keeps default colors.
#[test]
fn generated_component_stylesheets_do_not_leak_to_siblings() -> Result<()> {
    let mut component = MacroSiblingStyleRoot::new();
    let terminal = render_component(&mut component, 24, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "S").0, Color::Yellow);
    assert_eq!(rendered_cell_colors(&terminal, "P").0, Color::Reset);

    Ok(())
}

/// Verifies parent component styles apply through child component boundaries.
///
/// # Assertions
///
/// - A child component's text receives the parent component stylesheet.
#[test]
fn generated_component_stylesheets_apply_to_child_component_subtrees() -> Result<()> {
    let mut component = MacroParentStylesChild::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "P").0, Color::Green);

    Ok(())
}

/// Verifies child component styles are layered above parent component styles.
///
/// # Assertions
///
/// - A child text rule overrides the parent text rule for the child subtree.
#[test]
fn generated_child_component_stylesheet_overrides_parent_stylesheet() -> Result<()> {
    let mut component = MacroParentWithChildOverride::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "O").0, Color::Yellow);

    Ok(())
}

/// Verifies component stylesheets resolve against component-provided themes.
///
/// # Assertions
///
/// - A `theme_color(...)` declaration resolves from context during render.
#[test]
fn generated_component_stylesheet_resolves_theme_context() -> Result<()> {
    let mut component = MacroThemedStylesheet::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "T").0, Color::LightCyan);

    Ok(())
}

/// Verifies generated components dispatch key events through registered hooks.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn MacroButtonRoot() -> Node { use_key_event(Press, ...); button("Save").on_press(...) }
/// render, Repeat(s), Press(s)
/// ```
///
/// # Assertions
///
/// - The first render initializes the generated component's node tree.
/// - A repeat event does not invoke the press-only handler.
/// - A press event invokes the component key handler.
/// - Unhandled keys continue without invoking the handler.
///
/// # Why
///
/// Generated components should support custom key maps without requiring a
/// manual [`Component`] implementation.
#[test]
fn generated_components_dispatch_registered_key_handlers() -> Result<()> {
    MACRO_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let backend = TestBackend::new(16, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut component = MacroButtonRoot::new();
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = Component::render(&mut component, &mut ctx);
    })?;
    render_result?;

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('x')))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_BUTTON_PRESSES.load(Ordering::SeqCst), 0);

    assert_eq!(
        Component::handle_event(
            &mut component,
            key_with_kind(KeyCode::Char('s'), KeyEventKind::Repeat),
        )?,
        AppControl::Continue
    );
    assert_eq!(MACRO_BUTTON_PRESSES.load(Ordering::SeqCst), 0);

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('s')))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_BUTTON_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies key-event hooks can target repeat and release events explicitly.
///
/// # Example Under Test
///
/// ```text
/// use_key_event(Repeat, ...)
/// use_key_event(Release, ...)
/// Press(k), Repeat(k), Release(k)
/// ```
///
/// # Assertions
///
/// - The repeat handler ignores press and release events.
/// - The release handler ignores press and repeat events.
#[test]
fn generated_key_event_handlers_filter_by_event_kind() -> Result<()> {
    MACRO_REPEAT_KEY_PRESSES.store(0, Ordering::SeqCst);
    MACRO_RELEASE_KEY_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroKindSpecificKeyHandlers::new();

    assert_eq!(
        Component::handle_event(
            &mut component,
            key_with_kind(KeyCode::Char('k'), KeyEventKind::Press),
        )?,
        AppControl::Continue
    );
    assert_eq!(MACRO_REPEAT_KEY_PRESSES.load(Ordering::SeqCst), 0);
    assert_eq!(MACRO_RELEASE_KEY_PRESSES.load(Ordering::SeqCst), 0);

    assert_eq!(
        Component::handle_event(
            &mut component,
            key_with_kind(KeyCode::Char('k'), KeyEventKind::Repeat),
        )?,
        AppControl::Continue
    );
    assert_eq!(MACRO_REPEAT_KEY_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_RELEASE_KEY_PRESSES.load(Ordering::SeqCst), 0);

    assert_eq!(
        Component::handle_event(
            &mut component,
            key_with_kind(KeyCode::Char('k'), KeyEventKind::Release),
        )?,
        AppControl::Continue
    );
    assert_eq!(MACRO_REPEAT_KEY_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_RELEASE_KEY_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies generated components use default button keys when hooks pass.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn MacroDefaultButtonRoot() -> Node { button("Default").on_press(...) }
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - Tab focuses the generated button node.
/// - Enter activates the focused button.
#[test]
fn generated_components_run_default_button_keys_after_hook_pass() -> Result<()> {
    MACRO_DEFAULT_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroDefaultButtonRoot::new();

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_DEFAULT_BUTTON_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies local hooks can override default Tab focus movement.
///
/// # Example Under Test
///
/// ```text
/// use_key_event(Press, Tab => Handled)
/// button("Default").on_press(...)
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - Tab is consumed by the hook.
/// - Enter does not activate the button because focus did not move.
#[test]
fn generated_component_hook_can_override_default_tab_focus() -> Result<()> {
    MACRO_DEFAULT_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroTabOverrideButtonRoot::new();

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_DEFAULT_BUTTON_PRESSES.load(Ordering::SeqCst), 0);

    Ok(())
}

/// Verifies local hooks can override default Enter activation.
///
/// # Example Under Test
///
/// ```text
/// use_key_event(Press, Enter => Handled)
/// button("Default").on_press(...)
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - Tab uses the default focus behavior.
/// - Enter is consumed by the hook before the focused button activates.
#[test]
fn generated_component_hook_can_override_default_enter_activation() -> Result<()> {
    MACRO_DEFAULT_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroEnterOverrideButtonRoot::new();

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_DEFAULT_BUTTON_PRESSES.load(Ordering::SeqCst), 0);

    Ok(())
}

/// Verifies generated component setup owns persistent Leptos signal state.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn MacroSignalRoot() -> Node {
///     let (count, set_count) = signal(0);
///     column([dynamic(... count ...), button(... set_count ...)])
/// }
/// ```
///
/// # Assertions
///
/// - Component setup runs exactly once.
/// - The first render shows the initial signal value.
/// - A key event updates the signal.
/// - A redraw shows the updated signal without rerunning setup.
#[test]
fn generated_component_setup_runs_once_and_signals_persist() -> Result<()> {
    MACRO_SIGNAL_SETUP_RUNS.store(0, Ordering::SeqCst);

    let backend = TestBackend::new(24, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut component = MacroSignalRoot::new();

    assert_eq!(MACRO_SIGNAL_SETUP_RUNS.load(Ordering::SeqCst), 1);

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = Component::render(&mut component, &mut ctx);
    })?;
    render_result?;
    assert!(rendered_text(&terminal).contains("Count: 0"));

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('i')))?,
        AppControl::Continue
    );

    render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = Component::render(&mut component, &mut ctx);
    })?;
    render_result?;

    assert!(rendered_text(&terminal).contains("Count: 1"));
    assert_eq!(MACRO_SIGNAL_SETUP_RUNS.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies generated component key handlers can exit the app.
///
/// # Example Under Test
///
/// ```text
/// use_key_event(Press, |key| if key == q { KeyControl::Exit } else { KeyControl::Pass })
/// ```
///
/// # Assertions
///
/// - Unhandled keys continue the app.
/// - The `q` key returns [`AppControl::Exit`].
#[test]
fn generated_component_key_handler_can_exit() -> Result<()> {
    let mut component = MacroKeyExitRoot::new();

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('x')))?,
        AppControl::Continue
    );
    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('q')))?,
        AppControl::Exit
    );

    Ok(())
}

/// Verifies child key handlers override parent key handlers.
///
/// # Example Under Test
///
/// ```text
/// parent use_key_event(Press, x => Handled)
/// child use_key_event(Press, x => Handled)
/// ```
///
/// # Assertions
///
/// - The child handler observes `x`.
/// - The parent handler does not observe `x`.
#[test]
fn child_key_handler_overrides_parent_handler() -> Result<()> {
    MACRO_PARENT_KEY_PRESSES.store(0, Ordering::SeqCst);
    MACRO_CHILD_KEY_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroParentKeyRoot::new();

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('x')))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_CHILD_KEY_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_PARENT_KEY_PRESSES.load(Ordering::SeqCst), 0);

    Ok(())
}

/// Verifies child pass-through lets parent handlers run.
///
/// # Example Under Test
///
/// ```text
/// parent use_key_event(Press, p => Handled)
/// child use_key_event(Press, p => Pass)
/// ```
///
/// # Assertions
///
/// - The child handler observes `p`.
/// - The parent handler handles `p`.
#[test]
fn child_key_pass_reaches_parent_handler() -> Result<()> {
    MACRO_PASS_PARENT_KEY_PRESSES.store(0, Ordering::SeqCst);
    MACRO_PASS_CHILD_KEY_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroParentAfterPassRoot::new();

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('p')))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_PASS_CHILD_KEY_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_PASS_PARENT_KEY_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies handlers in one component run in registration order.
///
/// # Example Under Test
///
/// ```text
/// use_key_event(Press, m => Pass)
/// use_key_event(Press, m => Handled)
/// use_key_event(Press, m => Handled)
/// ```
///
/// # Assertions
///
/// - The first passing handler runs.
/// - The second handling callback runs.
/// - The third callback does not run.
#[test]
fn component_key_handlers_short_circuit_in_registration_order() -> Result<()> {
    MACRO_FIRST_KEY_HANDLER.store(0, Ordering::SeqCst);
    MACRO_SECOND_KEY_HANDLER.store(0, Ordering::SeqCst);
    MACRO_THIRD_KEY_HANDLER.store(0, Ordering::SeqCst);

    let mut component = MacroMultipleKeyHandlers::new();

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('m')))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_FIRST_KEY_HANDLER.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_SECOND_KEY_HANDLER.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_THIRD_KEY_HANDLER.load(Ordering::SeqCst), 0);

    Ok(())
}

/// Verifies generated component providers remain visible to descendants.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn MacroContextProvider() -> Node {
///     provide_context(MacroLabel("macro"));
///     component(MacroContextConsumer)
/// }
/// ```
///
/// # Assertions
///
/// - The generated provider renders successfully.
/// - The descendant component reads the context value provided during setup.
///
/// # Why
///
/// Generated component bodies run once under a stored Leptos owner whose
/// context remains active while rendering the returned node tree.
#[test]
fn generated_component_providers_are_visible_to_descendants() -> Result<()> {
    let backend = TestBackend::new(16, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut component = MacroContextProvider::new();

    for _ in 0..2 {
        *MACRO_CONTEXT_OBSERVED
            .lock()
            .expect("context observation lock should be available") = None;

        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            render_result = Component::render(&mut component, &mut ctx);
        })?;
        render_result?;

        assert_eq!(
            *MACRO_CONTEXT_OBSERVED
                .lock()
                .expect("context observation lock should be available"),
            Some(MacroLabel("macro"))
        );
    }

    Ok(())
}

/// Verifies macros compile when the runtime crate dependency is renamed.
///
/// # Example Under Test
///
/// ```text
/// ui = { package = "leptatui", path = "..." }
/// use ui::prelude::*;
/// #[component]
/// fn Greeting() -> Node { view! { <Text>"hi"</Text> } }
/// ```
///
/// # Assertions
///
/// - `cargo check` succeeds in a temporary downstream crate.
/// - The downstream crate imports only the renamed `ui` dependency.
///
/// # Why
///
/// Generated proc-macro code should resolve the runtime crate path from the
/// caller's dependency name instead of hardcoding `::leptatui`.
#[test]
fn macros_compile_with_renamed_runtime_dependency() {
    let project_dir = create_alias_fixture();
    let output = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .current_dir(&project_dir)
        .output()
        .expect("cargo check should run for alias fixture");

    assert!(
        output.status.success(),
        "cargo check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Creates a temporary crate that depends on Leptatui under a renamed package key.
///
/// # Returns
///
/// A [`PathBuf`] containing the generated fixture crate directory.
fn create_alias_fixture() -> PathBuf {
    let project_dir = alias_fixture_dir();
    if project_dir.exists() {
        fs::remove_dir_all(&project_dir).expect("stale alias fixture should be removable");
    }

    fs::create_dir_all(project_dir.join("src")).expect("alias fixture src should be creatable");

    let leptatui_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("leptatui manifest directory should be canonicalizable");

    fs::write(
        project_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "leptatui-alias-check"
version = "0.0.0"
edition = "2024"

[dependencies]
ui = {{ package = "leptatui", path = "{}" }}
"#,
            leptatui_path.display()
        ),
    )
    .expect("alias fixture manifest should be writable");

    fs::write(
        project_dir.join("src/main.rs"),
        r#"use ui::prelude::*;

#[component]
fn Greeting() -> Node {
    view! { <Text>"hi"</Text> }
}

fn main() {
    let node: Node = Greeting::new().into();
    assert!(matches!(node, Node::Component(_)));
}
"#,
    )
    .expect("alias fixture source should be writable");

    project_dir
}

/// Returns a unique temporary directory for a renamed-dependency fixture crate.
///
/// # Returns
///
/// A [`PathBuf`] under Cargo's target temp directory or the system temp
/// directory.
fn alias_fixture_dir() -> PathBuf {
    let base = std::env::var_os("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after the Unix epoch")
        .as_nanos();

    base.join(format!(
        "leptatui-alias-check-{}-{timestamp}",
        std::process::id()
    ))
}
