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
    AppControl, Children, Color, Component, KeyControl, LayoutDirection, RenderCtx, Result,
    ThemeVariables, button, column, component, dynamic, row, stylesheet, text, theme_color,
    use_key_event, view,
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
static MACRO_FIRST_WRAPPED_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_SECOND_WRAPPED_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_MIXED_BUILTIN_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_MIXED_WRAPPED_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_REPEAT_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_RELEASE_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);

/// Context value used by generated component provider tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MacroLabel(&'static str);

/// Component with a local stylesheet applied to its own text view.
#[component]
fn MacroStyledText() -> leptatui::View {
    stylesheet! {
        .scoped => { fg: Color::Yellow, bg: Color::Blue }
    }

    text("Scoped").with_classes("scoped")
}

/// Component whose stylesheet targets a shared class name.
#[component]
fn MacroStyledSibling() -> leptatui::View {
    stylesheet! {
        .shared => { fg: Color::Yellow }
    }

    text("Styled").with_classes("shared")
}

/// Component with a class that should not receive sibling styles.
#[component]
fn MacroPlainSibling() -> leptatui::View {
    text("Plain").with_classes("shared")
}

/// Parent component whose stylesheet should apply to child component internals.
#[component]
fn MacroParentStylesChild() -> leptatui::View {
    stylesheet! {
        Text => { fg: Color::Green }
    }

    component(MacroPlainSibling::new())
}

/// Parent and child components with equal-specificity text rules.
#[component]
fn MacroParentWithChildOverride() -> leptatui::View {
    stylesheet! {
        Text => { fg: Color::Green }
    }

    component(MacroChildStyleOverride::new())
}

/// Child component whose equal-specificity stylesheet should be later in source order.
#[component]
fn MacroChildStyleOverride() -> leptatui::View {
    stylesheet! {
        Text => { fg: Color::Yellow }
    }

    text("Override")
}

/// Parent component with a class rule that should beat a child type rule.
#[component]
fn MacroParentSpecificityBeatsChild() -> leptatui::View {
    stylesheet! {
        .specific => { fg: Color::Green }
    }

    component(MacroChildLowerSpecificity::new())
}

/// Child component with a lower-specificity type rule.
#[component]
fn MacroChildLowerSpecificity() -> leptatui::View {
    stylesheet! {
        Text => { fg: Color::Yellow }
    }

    text("Specific").with_classes("specific")
}

/// Component whose stylesheet resolves against theme context it provides.
#[component]
fn MacroThemedStylesheet() -> leptatui::View {
    provide_context(ThemeVariables::new().color("text", Color::LightCyan));

    stylesheet! {
        .themed => { fg: theme_color("text") }
    }

    text("Theme").with_classes("themed")
}

/// Component that renders a required prop.
#[component]
fn MacroPropLabel(#[prop(into)] label: String) -> leptatui::View {
    text(label)
}

/// Component that renders a prop and nested children.
#[component]
fn MacroPropPanel(#[prop(into)] title: String, children: Children) -> leptatui::View {
    column([text(title), column(children())])
}

/// Component whose internal layout changes height under a media rule.
#[component]
fn MacroResponsiveCaseRow() -> leptatui::View {
    view! {
        <Row class="case-row">
            <Text>"type < class"</Text>
            <Text>"Sample"</Text>
        </Row>
    }
}

/// Parent component that must reserve the responsive child component height.
#[component]
fn MacroResponsiveCaseRoot() -> leptatui::View {
    stylesheet! {
        @media (max-width: 60) {
            .case-row => { direction: LayoutDirection::Column }
        }
    }

    view! {
        <Column>
            <Text>"Intro"</Text>
            <MacroResponsiveCaseRow />
        </Column>
    }
}

/// Component with an overflowing internal layout.
#[component]
fn MacroScrollableList() -> leptatui::View {
    column([
        text("One"),
        text("Two"),
        text("Three"),
        text("Four"),
        text("Five"),
        text("Six"),
    ])
}

/// Parent component whose default scroll keys must reach a child component.
#[component]
fn MacroScrollableBoundaryRoot() -> leptatui::View {
    row([component(MacroScrollableList::new())])
}

/// Parent component with styled and plain sibling component subtrees.
#[component]
fn MacroSiblingStyleRoot() -> leptatui::View {
    row([
        component(MacroStyledSibling::new()),
        component(MacroPlainSibling::new()),
    ])
}

/// Component with an interactive button used by macro runtime tests.
#[component]
fn MacroButtonRoot() -> leptatui::View {
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
fn MacroDefaultButtonRoot() -> leptatui::View {
    button("Default").on_press(|| {
        MACRO_DEFAULT_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}

/// Component that wraps one built-in button.
#[component]
fn MacroWrappedButton(#[prop(into)] label: String, on_press: fn() -> AppControl) -> leptatui::View {
    button(label).on_press(on_press)
}

/// Root with sibling custom button components.
#[component]
fn MacroWrappedButtonSiblings() -> leptatui::View {
    view! {
        <Row>
            <MacroWrappedButton label="First" on_press=macro_first_wrapped_button_press />
            <MacroWrappedButton label="Second" on_press=macro_second_wrapped_button_press />
        </Row>
    }
}

/// Root with a built-in button and a custom button component.
#[component]
fn MacroMixedButtonSiblings() -> leptatui::View {
    view! {
        <Row>
            <Button on_press={macro_mixed_builtin_button_press}>"Built in"</Button>
            <MacroWrappedButton label="Wrapped" on_press=macro_mixed_wrapped_button_press />
        </Row>
    }
}

/// Component whose key map handles Tab before focus can move.
#[component]
fn MacroTabOverrideButtonRoot() -> leptatui::View {
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
fn MacroEnterOverrideButtonRoot() -> leptatui::View {
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
fn MacroSignalRoot() -> leptatui::View {
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
fn MacroContextProvider() -> leptatui::View {
    leptatui::context::provide_context(MacroLabel("macro"));
    component(MacroContextConsumer)
}

/// Component that exits when `q` is pressed.
#[component]
fn MacroKeyExitRoot() -> leptatui::View {
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
fn MacroParentKeyRoot() -> leptatui::View {
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
fn MacroChildKeyHandler() -> leptatui::View {
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
fn MacroParentAfterPassRoot() -> leptatui::View {
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
fn MacroPassingChildKeyHandler() -> leptatui::View {
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
fn MacroMultipleKeyHandlers() -> leptatui::View {
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
fn MacroKindSpecificKeyHandlers() -> leptatui::View {
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

/// Records activation for the first wrapped button.
fn macro_first_wrapped_button_press() -> AppControl {
    MACRO_FIRST_WRAPPED_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
    AppControl::Continue
}

/// Records activation for the second wrapped button.
fn macro_second_wrapped_button_press() -> AppControl {
    MACRO_SECOND_WRAPPED_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
    AppControl::Continue
}

/// Records activation for the mixed built-in button.
fn macro_mixed_builtin_button_press() -> AppControl {
    MACRO_MIXED_BUILTIN_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
    AppControl::Continue
}

/// Records activation for the mixed wrapped button.
fn macro_mixed_wrapped_button_press() -> AppControl {
    MACRO_MIXED_WRAPPED_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
    AppControl::Continue
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

/// Verifies generated props are available while the component tree is built.
///
/// # Assertions
///
/// - A required `into` prop renders as text.
/// - Nested children passed through a `Children` prop render inside the panel.
#[test]
fn generated_component_props_render() -> Result<()> {
    let mut component = MacroPropPanel::with_props(
        MacroPropPanelProps::builder()
            .title("Panel")
            .children(Box::new(|| {
                vec![
                    MacroPropLabel::with_props(
                        MacroPropLabelProps::builder().label("Child").build(),
                    )
                    .into(),
                ]
            }))
            .build(),
    );
    let terminal = render_component(&mut component, 24, 6)?;
    let text = rendered_text(&terminal);

    assert!(text.contains("Panel"), "rendered text: {text:?}");
    assert!(text.contains("Child"), "rendered text: {text:?}");

    Ok(())
}

/// Verifies generated component boundaries report responsive internal height.
#[test]
fn generated_component_min_height_tracks_responsive_internal_layout() -> Result<()> {
    let mut component = MacroResponsiveCaseRoot::new();
    let terminal = render_component(&mut component, 40, 3)?;
    let text = rendered_text(&terminal);

    assert!(text.contains("Intro"), "rendered text: {text:?}");
    assert!(text.contains("type < class"), "rendered text: {text:?}");
    assert!(text.contains("Sample"), "rendered text: {text:?}");

    Ok(())
}

/// Verifies default scroll keys cross generated component boundaries.
///
/// # Example Under Test
///
/// ```text
/// Row(<MacroScrollableList />)
/// PageDown, gg, G
/// ```
///
/// # Assertions
///
/// - The initial render shows the top of the child component list.
/// - PageDown scrolls the child component's overflowing column.
/// - `gg` returns the child component's overflowing column to the top.
/// - `G` scrolls the child component's overflowing column to the bottom.
#[test]
fn generated_component_scroll_keys_cross_component_boundaries() -> Result<()> {
    let mut component = MacroScrollableBoundaryRoot::new();
    let terminal = render_component(&mut component, 12, 3)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("One"), "rendered text: {text:?}");
    assert!(!text.contains("Six"), "rendered text: {text:?}");

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::PageDown))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 12, 3)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Six"), "rendered text: {text:?}");

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('g')))?,
        AppControl::Continue
    );
    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('g')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 12, 3)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("One"), "rendered text: {text:?}");
    assert!(!text.contains("Six"), "rendered text: {text:?}");

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('G')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 12, 3)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Six"), "rendered text: {text:?}");

    Ok(())
}

/// Verifies a bare `stylesheet!` statement registers with its component.
///
/// # Assertions
///
/// - The component's text receives the foreground and background from the
///   local stylesheet.
#[test]
fn generated_component_stylesheet_styles_own_views() -> Result<()> {
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

/// Verifies equal-specificity child component styles win by source order.
///
/// # Assertions
///
/// - A child text rule overrides an equal-specificity parent text rule.
#[test]
fn generated_equal_specificity_child_stylesheet_wins_by_source_order() -> Result<()> {
    let mut component = MacroParentWithChildOverride::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "O").0, Color::Yellow);

    Ok(())
}

/// Verifies parent component specificity participates in the CSS cascade.
///
/// # Assertions
///
/// - A parent class rule overrides a lower-specificity child text rule.
#[test]
fn generated_higher_specificity_parent_stylesheet_overrides_child_stylesheet() -> Result<()> {
    let mut component = MacroParentSpecificityBeatsChild::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "S").0, Color::Green);

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
/// fn MacroButtonRoot() -> View { use_key_event(Press, ...); button("Save").on_press(...) }
/// render, Repeat(s), Press(s)
/// ```
///
/// # Assertions
///
/// - The first render initializes the generated component's view tree.
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
/// fn MacroDefaultButtonRoot() -> View { button("Default").on_press(...) }
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - Tab focuses the generated button view.
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

/// Verifies default focus traversal crosses generated component boundaries.
///
/// # Example Under Test
///
/// ```text
/// Row(<WrappedButton first />, <WrappedButton second />)
/// Tab, Enter, Tab, Enter, BackTab, Enter
/// ```
///
/// # Assertions
///
/// - The first Tab focuses the first wrapped button.
/// - The second Tab focuses the second wrapped button.
/// - BackTab returns focus to the first wrapped button.
#[test]
fn generated_component_focus_crosses_sibling_component_boundaries() -> Result<()> {
    MACRO_FIRST_WRAPPED_BUTTON_PRESSES.store(0, Ordering::SeqCst);
    MACRO_SECOND_WRAPPED_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroWrappedButtonSiblings::new();

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_FIRST_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(
        MACRO_SECOND_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst),
        0
    );

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_FIRST_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(
        MACRO_SECOND_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst),
        1
    );

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::BackTab))?,
        AppControl::Continue
    );
    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_FIRST_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 2);
    assert_eq!(
        MACRO_SECOND_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst),
        1
    );

    Ok(())
}

/// Verifies built-in buttons and component-wrapped buttons share focus order.
#[test]
fn generated_component_focus_mixes_static_and_component_buttons() -> Result<()> {
    MACRO_MIXED_BUILTIN_BUTTON_PRESSES.store(0, Ordering::SeqCst);
    MACRO_MIXED_WRAPPED_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroMixedButtonSiblings::new();

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_MIXED_BUILTIN_BUTTON_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_MIXED_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 0);

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_MIXED_BUILTIN_BUTTON_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_MIXED_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 1);

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
/// fn MacroSignalRoot() -> View {
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
/// fn MacroContextProvider() -> View {
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
/// context remains active while rendering the returned view tree.
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
/// fn Greeting() -> View { view! { <Text>"hi"</Text> } }
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
fn Greeting() -> View {
    view! { <Text>"hi"</Text> }
}

fn main() {
    let view: View = Greeting::new().into();
    assert!(matches!(view, View::Component(_)));
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
