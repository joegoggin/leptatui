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
    AppControl, Children, Color, KeyControl, LayoutDirection, RenderCtx, Result, ThemeVariables,
    View, button, column, component, dynamic, row, stylesheet, text, theme_color, use_key_event,
    view,
};
use leptos::prelude::{GetUntracked, ReadSignal, Update, signal};
use ratatui::{Terminal, backend::TestBackend};

use crate::support::{key, render_component, rendered_text};

/// Button press count observed by basic macro component tests.
static MACRO_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Setup run count observed by signal-backed macro component tests.
static MACRO_SIGNAL_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
/// Context value observed by generated component context tests.
static MACRO_CONTEXT_OBSERVED: Mutex<Option<MacroLabel>> = Mutex::new(None);
/// Parent key handler count for nested key propagation tests.
static MACRO_PARENT_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Child key handler count for nested key propagation tests.
static MACRO_CHILD_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Parent key handler count for pass-through key propagation tests.
static MACRO_PASS_PARENT_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Child key handler count for pass-through key propagation tests.
static MACRO_PASS_CHILD_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// First local key handler count for source-order dispatch tests.
static MACRO_FIRST_KEY_HANDLER: AtomicUsize = AtomicUsize::new(0);
/// Second local key handler count for source-order dispatch tests.
static MACRO_SECOND_KEY_HANDLER: AtomicUsize = AtomicUsize::new(0);
/// Third local key handler count for source-order dispatch tests.
static MACRO_THIRD_KEY_HANDLER: AtomicUsize = AtomicUsize::new(0);
/// Built-in default button activation count for key dispatch tests.
static MACRO_DEFAULT_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// First wrapped button activation count for component-boundary tests.
static MACRO_FIRST_WRAPPED_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Second wrapped button activation count for component-boundary tests.
static MACRO_SECOND_WRAPPED_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Built-in button activation count for mixed child key dispatch tests.
static MACRO_MIXED_BUILTIN_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Wrapped button activation count for mixed child key dispatch tests.
static MACRO_MIXED_WRAPPED_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Repeated-key handler count for key-kind filtering tests.
static MACRO_REPEAT_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Release-key handler count for key-kind filtering tests.
static MACRO_RELEASE_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Root route component setup count for route-switching tests.
static MACRO_ROUTE_ROOT_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
/// Home route component setup count for route-switching tests.
static MACRO_ROUTE_HOME_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
/// Counter route component setup count for route-switching tests.
static MACRO_ROUTE_COUNTER_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
/// Settings route component setup count for route-switching tests.
static MACRO_ROUTE_SETTINGS_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);

/// Context value used by generated component provider tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MacroLabel(&'static str);

/// Shared root-owned state exposed to route page branches.
#[derive(Clone, Copy)]
struct MacroSharedCount(ReadSignal<usize>);

/// Route values used by route-driven page switching tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MacroRoutePage {
    Home,
    Counter,
    Settings,
}

/// View with a local stylesheet applied to its own text view.
#[component]
fn MacroStyledText() -> impl leptatui::IntoView {
    stylesheet! {
        .scoped => { fg: Color::Yellow, bg: Color::Blue }
    }

    text("Scoped").with_classes("scoped")
}

/// View whose stylesheet targets a shared class name.
#[component]
fn MacroStyledSibling() -> impl leptatui::IntoView {
    stylesheet! {
        .shared => { fg: Color::Yellow }
    }

    text("Styled").with_classes("shared")
}

/// View with a class that should not receive sibling styles.
#[component]
fn MacroPlainSibling() -> impl leptatui::IntoView {
    text("Plain").with_classes("shared")
}

/// Parent component whose stylesheet should apply to child component internals.
#[component]
fn MacroParentStylesChild() -> impl leptatui::IntoView {
    stylesheet! {
        Text => { fg: Color::Green }
    }

    component(MacroPlainSibling::new())
}

/// Parent and child components with equal-specificity text rules.
#[component]
fn MacroParentWithChildOverride() -> impl leptatui::IntoView {
    stylesheet! {
        Text => { fg: Color::Green }
    }

    component(MacroChildStyleOverride::new())
}

/// Child component whose equal-specificity stylesheet should be later in source order.
#[component]
fn MacroChildStyleOverride() -> impl leptatui::IntoView {
    stylesheet! {
        Text => { fg: Color::Yellow }
    }

    text("Override")
}

/// Parent component with a class rule that should beat a child type rule.
#[component]
fn MacroParentSpecificityBeatsChild() -> impl leptatui::IntoView {
    stylesheet! {
        .specific => { fg: Color::Green }
    }

    component(MacroChildLowerSpecificity::new())
}

/// Child component with a lower-specificity type rule.
#[component]
fn MacroChildLowerSpecificity() -> impl leptatui::IntoView {
    stylesheet! {
        Text => { fg: Color::Yellow }
    }

    text("Specific").with_classes("specific")
}

/// View whose stylesheet resolves against theme context it provides.
#[component]
fn MacroThemedStylesheet() -> impl leptatui::IntoView {
    provide_context(ThemeVariables::new().color("text", Color::LightCyan));

    stylesheet! {
        .themed => { fg: theme_color("text") }
    }

    text("Theme").with_classes("themed")
}

/// View that renders a required prop.
#[component]
fn MacroPropLabel(#[prop(into)] label: String) -> impl leptatui::IntoView {
    text(label)
}

/// View that renders a prop and nested children.
#[component]
fn MacroPropPanel(#[prop(into)] title: String, children: Children) -> impl leptatui::IntoView {
    column((text(title), column(children())))
}

/// View whose internal layout changes height under a media rule.
#[component]
fn MacroResponsiveCaseRow() -> impl leptatui::IntoView {
    view! {
        <Row class="case-row">
            <Text>"type < class"</Text>
            <Text>"Sample"</Text>
        </Row>
    }
}

/// Parent component that must reserve the responsive child component height.
#[component]
fn MacroResponsiveCaseRoot() -> impl leptatui::IntoView {
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

/// View with an overflowing internal layout.
#[component]
fn MacroScrollableList() -> impl leptatui::IntoView {
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
fn MacroScrollableBoundaryRoot() -> impl leptatui::IntoView {
    row([component(MacroScrollableList::new())])
}

/// Parent component with styled and plain sibling component subtrees.
#[component]
fn MacroSiblingStyleRoot() -> impl leptatui::IntoView {
    row([
        component(MacroStyledSibling::new()),
        component(MacroPlainSibling::new()),
    ])
}

/// View with an interactive button used by macro runtime tests.
#[component]
fn MacroButtonRoot() -> impl leptatui::IntoView {
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

/// View with no matching hook for default button key tests.
#[component]
fn MacroDefaultButtonRoot() -> impl leptatui::IntoView {
    button("Default").on_press(|| {
        MACRO_DEFAULT_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}

/// View that wraps one built-in button.
#[component]
fn MacroWrappedButton(
    #[prop(into)] label: String,
    on_press: fn() -> AppControl,
) -> impl leptatui::IntoView {
    button(label).on_press(on_press)
}

/// Root with sibling custom button components.
#[component]
fn MacroWrappedButtonSiblings() -> impl leptatui::IntoView {
    view! {
        <Row>
            <MacroWrappedButton label="First" on_press=macro_first_wrapped_button_press />
            <MacroWrappedButton label="Second" on_press=macro_second_wrapped_button_press />
        </Row>
    }
}

/// Root with a built-in button and a custom button component.
#[component]
fn MacroMixedButtonSiblings() -> impl leptatui::IntoView {
    view! {
        <Row>
            <Button on_press={macro_mixed_builtin_button_press}>"Built in"</Button>
            <MacroWrappedButton label="Wrapped" on_press=macro_mixed_wrapped_button_press />
        </Row>
    }
}

/// View whose key map handles Tab before focus can move.
#[component]
fn MacroTabOverrideButtonRoot() -> impl leptatui::IntoView {
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

/// View whose key map handles Enter before a focused button activates.
#[component]
fn MacroEnterOverrideButtonRoot() -> impl leptatui::IntoView {
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

/// View with local signal state created during generated setup.
#[component]
fn MacroSignalRoot() -> impl leptatui::IntoView {
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

    column((
        dynamic(move || text(format!("Count: {}", count.get_untracked()))),
        button("Increment").on_press(move || {
            set_count.update(|count| *count += 1);
            AppControl::Continue
        }),
    ))
}

/// Root component that renders page branches from route state.
#[component]
fn MacroRouteSwitchRoot() -> impl leptatui::IntoView {
    MACRO_ROUTE_ROOT_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);

    let (shared_count, set_shared_count) = signal(0);
    provide_context(MacroSharedCount(shared_count));
    let route_state = leptatui::provide_route(MacroRoutePage::Home);
    let route = route_state.route();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('i') {
            set_shared_count.update(|count| *count += 1);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    view! {
        <Column>
            <MacroRouteKeyNav />
            {move || match route.get_untracked() {
                MacroRoutePage::Home => view! { <MacroRouteHomePage /> },
                MacroRoutePage::Counter => view! { <MacroRouteCounterPage /> },
                MacroRoutePage::Settings => view! { <MacroRouteSettingsPage /> },
            }}
        </Column>
    }
}

/// Descendant component that navigates by updating route context.
#[component]
fn MacroRouteKeyNav() -> impl leptatui::IntoView {
    let navigate = leptatui::use_navigate::<MacroRoutePage>();

    use_key_event(KeyEventKind::Press, move |key| {
        match key.code {
            KeyCode::Char('h') => navigate.update(|route| *route = MacroRoutePage::Home),
            KeyCode::Char('c') => navigate.update(|route| *route = MacroRoutePage::Counter),
            KeyCode::Char('s') => navigate.update(|route| *route = MacroRoutePage::Settings),
            _ => return KeyControl::Pass,
        }

        KeyControl::Handled
    });

    text("Route keys")
}

/// Home page branch for route switching tests.
#[component]
fn MacroRouteHomePage() -> impl leptatui::IntoView {
    MACRO_ROUTE_HOME_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);
    let shared = leptatui::context::expect_context::<MacroSharedCount>().0;

    view! {
        <Column>
            {move || text(format!("Home {}", shared.get_untracked()))}
        </Column>
    }
}

/// Counter page branch for route switching tests.
#[component]
fn MacroRouteCounterPage() -> impl leptatui::IntoView {
    MACRO_ROUTE_COUNTER_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);
    let shared = leptatui::context::expect_context::<MacroSharedCount>().0;

    view! {
        <Column>
            {move || text(format!("Counter {}", shared.get_untracked()))}
        </Column>
    }
}

/// Settings page branch for route switching tests.
#[component]
fn MacroRouteSettingsPage() -> impl leptatui::IntoView {
    MACRO_ROUTE_SETTINGS_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);
    let shared = leptatui::context::expect_context::<MacroSharedCount>().0;

    view! {
        <Column>
            {move || text(format!("Settings {}", shared.get_untracked()))}
        </Column>
    }
}

/// Root component that switches between branches using the same component type.
#[component]
fn MacroRoutePropSwitchRoot() -> impl leptatui::IntoView {
    let route_state = leptatui::provide_route(MacroRoutePage::Home);
    let route = route_state.route();
    let navigate = route_state.navigate();

    use_key_event(KeyEventKind::Press, move |key| {
        match key.code {
            KeyCode::Char('h') => navigate.update(|route| *route = MacroRoutePage::Home),
            KeyCode::Char('c') => navigate.update(|route| *route = MacroRoutePage::Counter),
            KeyCode::Char('s') => navigate.update(|route| *route = MacroRoutePage::Settings),
            _ => return KeyControl::Pass,
        }

        KeyControl::Handled
    });

    view! {
        <Column>
            {move || match route.get_untracked() {
                MacroRoutePage::Home => view! { <MacroRouteNamedPage label="Home" /> },
                MacroRoutePage::Counter => view! { <MacroRouteNamedPage label="Counter" /> },
                MacroRoutePage::Settings => view! { <MacroRouteNamedPage label="Settings" /> },
            }}
        </Column>
    }
}

/// Route page whose prop must update when branches share this component type.
#[component]
fn MacroRouteNamedPage(#[prop(into)] label: String) -> impl leptatui::IntoView {
    view! { <Text>{label}</Text> }
}

/// View that records the label visible from its render context.
struct MacroContextConsumer;

impl View for MacroContextConsumer {
    /// Records the context label visible during render.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        *MACRO_CONTEXT_OBSERVED
            .lock()
            .expect("context observation lock should be available") =
            leptatui::context::use_context::<MacroLabel>();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// View that provides context to a descendant component boundary.
#[component]
fn MacroContextProvider() -> impl leptatui::IntoView {
    leptatui::context::provide_context(MacroLabel("macro"));
    component(MacroContextConsumer)
}

/// View that exits when `q` is pressed.
#[component]
fn MacroKeyExitRoot() -> impl leptatui::IntoView {
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
fn MacroParentKeyRoot() -> impl leptatui::IntoView {
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
fn MacroChildKeyHandler() -> impl leptatui::IntoView {
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
fn MacroParentAfterPassRoot() -> impl leptatui::IntoView {
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
fn MacroPassingChildKeyHandler() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('p') {
            MACRO_PASS_CHILD_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
        }

        KeyControl::Pass
    });

    text("Child")
}

/// View with several handlers used to prove source-order short-circuiting.
#[component]
fn MacroMultipleKeyHandlers() -> impl leptatui::IntoView {
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

/// View with explicit repeat and release key handlers.
#[component]
fn MacroKindSpecificKeyHandlers() -> impl leptatui::IntoView {
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

include!("alias.rs");
include!("compile.rs");
include!("context.rs");
include!("input.rs");
include!("key_events.rs");
include!("lifecycle.rs");
include!("rendering.rs");
include!("routing.rs");
include!("styling.rs");
