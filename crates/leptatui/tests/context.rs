//! Context storage tests.
//!
//! These tests cover public context APIs at runtime boundaries.

use leptatui::{
    AppControl, AppRoot, Color, Component, RenderCtx, Result, StyleDeclarations, StyleSelector,
    Stylesheet, ThemeVariables, column, component,
    context::{expect_context, provide_context, use_context},
    text, theme_color,
};
use leptos::prelude::{GetUntracked, Owner, ReadSignal, Set, signal};
use ratatui::{Terminal, backend::TestBackend};
use std::{cell::RefCell, rc::Rc};

/// Context value used by subtree scope tests.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ScopeLabel(&'static str);

/// Shared observations captured by test components.
type ObservedLabels = Rc<RefCell<Vec<Option<ScopeLabel>>>>;

/// Component that provides a label to its child subtree.
struct LabelProvider {
    value: ScopeLabel,
    child: leptatui::Node,
}

impl Component for LabelProvider {
    /// Provides a label, then renders the child subtree.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        provide_context(self.value.clone());
        ctx.render_node(&self.child)
    }
}

/// Component that records the label visible from its current context scope.
struct LabelConsumer {
    observed: ObservedLabels,
}

impl LabelConsumer {
    /// Creates a consumer writing into shared observations.
    fn new(observed: ObservedLabels) -> Self {
        Self { observed }
    }
}

impl Component for LabelConsumer {
    /// Records the visible label during render.
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        self.observed.borrow_mut().push(use_context::<ScopeLabel>());
        Ok(())
    }
}

/// Component that provides a label during render and forwards events to a child.
struct EventLabelProvider {
    value: ScopeLabel,
    child: leptatui::Node,
}

impl Component for EventLabelProvider {
    /// Provides a label while rendering the provider subtree.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        provide_context(self.value.clone());
        ctx.render_node(&self.child)
    }

    /// Forwards events to the child while this provider's scope is active.
    fn handle_event(&mut self, event: crossterm::event::Event) -> Result<AppControl> {
        self.child.handle_event(event)
    }
}

/// Component that records context visible during event handling.
struct EventLabelConsumer {
    observed: Rc<RefCell<Option<ScopeLabel>>>,
}

impl Component for EventLabelConsumer {
    /// Renders nothing; this component only observes event-time context.
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }

    /// Records the label visible during event handling.
    fn handle_event(&mut self, _event: crossterm::event::Event) -> Result<AppControl> {
        *self.observed.borrow_mut() = use_context::<ScopeLabel>();
        Ok(AppControl::Continue)
    }
}

/// Component that provides active theme variables before rendering a child.
struct ThemeRenderRoot {
    dark: ReadSignal<bool>,
    child: leptatui::Node,
    stylesheet: Stylesheet,
}

impl Component for ThemeRenderRoot {
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let theme = if self.dark.get_untracked() {
            ThemeVariables::new()
                .color("text", Color::White)
                .color("surface", Color::Black)
        } else {
            ThemeVariables::new()
                .color("text", Color::Black)
                .color("surface", Color::White)
        };

        provide_context(theme);
        ctx.__with_stylesheet(&self.stylesheet, |ctx| ctx.render_node(&self.child))
    }
}

/// Component that provides active theme variables through a signal context.
struct ThemeSignalRoot {
    theme: ReadSignal<ThemeVariables>,
    child: leptatui::Node,
    stylesheet: Stylesheet,
}

impl Component for ThemeSignalRoot {
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        provide_context(self.theme);
        ctx.__with_stylesheet(&self.stylesheet, |ctx| ctx.render_node(&self.child))
    }
}

/// Component that records context values observed during rendering.
struct ContextRoot {
    /// String context observed through Leptatui render-scope storage.
    observed_text: Option<String>,
    /// Signal value observed through Leptos owner fallback storage.
    observed_count: Option<i32>,
}

impl Component for ContextRoot {
    /// Provides and reads context values during a render pass.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Rendering context supplied by the app root adapter.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        provide_context(String::from("from component"));
        self.observed_text = use_context::<String>();

        Owner::new().with(|| {
            let (count, set_count) = signal(1);
            leptos::context::provide_context(count);
            set_count.set(2);

            self.observed_count =
                use_context::<ReadSignal<i32>>().map(|count| count.get_untracked());
        });

        Ok(())
    }
}

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
        observed_text: None,
        observed_count: None,
    };
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        render_result = AppRoot::render(&mut root, frame);
    })?;
    render_result?;

    assert_eq!(root.observed_text.as_deref(), Some("from component"));
    assert_eq!(root.observed_count, Some(2));

    Ok(())
}

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
        child: text("Theme").with_classes("themed"),
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
        child: text("Theme").with_classes("themed"),
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

/// Verifies Leptos owner context remains available through Leptatui lookup.
///
/// # Example Under Test
///
/// ```text
/// Owner::new().with(|| {
///     leptos::context::provide_context(String::from("from leptos"));
///     use_context::<String>()
/// })
/// ```
///
/// # Assertions
///
/// - `use_context` finds the Leptos-provided string.
/// - `expect_context` returns the Leptos-provided string.
///
/// # Why
///
/// Leptatui context helpers should not break existing Leptos owner context.
#[test]
fn leptos_owner_context_fallback_still_works() {
    Owner::new().with(|| {
        leptos::context::provide_context(String::from("from leptos"));

        assert_eq!(use_context::<String>().as_deref(), Some("from leptos"));
        assert_eq!(expect_context::<String>(), "from leptos");
    });
}

/// Verifies context follows component subtree ancestry during rendering.
///
/// # Example Under Test
///
/// ```text
/// outer provider
///   consumer -> outer
///   inner provider
///     consumer -> inner
///   consumer -> outer
/// ```
///
/// # Assertions
///
/// - The first descendant sees the outer provider value.
/// - The inner descendant sees the inner provider value.
/// - The sibling after the inner provider sees the restored outer value.
#[test]
fn component_context_is_scoped_to_render_subtrees() -> Result<()> {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let backend = TestBackend::new(24, 6);
    let mut terminal = Terminal::new(backend)?;
    let node = component(LabelProvider {
        value: ScopeLabel("outer"),
        child: column([
            component(LabelConsumer::new(Rc::clone(&observed))),
            component(LabelProvider {
                value: ScopeLabel("inner"),
                child: component(LabelConsumer::new(Rc::clone(&observed))),
            }),
            component(LabelConsumer::new(Rc::clone(&observed))),
        ]),
    });
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = node.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(
        observed.borrow().as_slice(),
        [
            Some(ScopeLabel("outer")),
            Some(ScopeLabel("inner")),
            Some(ScopeLabel("outer")),
        ]
    );

    Ok(())
}

/// Verifies provider ancestry is available during descendant event handling.
///
/// # Example Under Test
///
/// ```text
/// render provider -> stores ScopeLabel("event")
/// dispatch event through same provider subtree
/// child event handler reads ScopeLabel("event")
/// ```
///
/// # Assertions
///
/// - The initial render succeeds.
/// - Event traversal continues.
/// - The child event handler sees the provider value from the latest render.
#[test]
fn component_context_is_available_during_descendant_events() -> Result<()> {
    let observed = Rc::new(RefCell::new(None));
    let backend = TestBackend::new(24, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut node = component(EventLabelProvider {
        value: ScopeLabel("event"),
        child: component(EventLabelConsumer {
            observed: Rc::clone(&observed),
        }),
    });
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = node.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(
        node.handle_event(crossterm::event::Event::Resize(24, 4))?,
        AppControl::Continue
    );
    assert_eq!(*observed.borrow(), Some(ScopeLabel("event")));

    Ok(())
}
