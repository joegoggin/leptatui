// Shared context test fixtures and imports.

use leptatui::{
    AnyView, AppControl, AppRoot, Color, IntoView, RenderCtx, StyleDeclarations,
    StyleSelector, Stylesheet, ThemeVariables, View, component, div,
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
    child: AnyView,
}

impl View for LabelProvider {
    /// Provides a label, then renders the child subtree.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> leptatui::app::Result<()> {
        provide_context(self.value.clone());
        ctx.render_view(&self.child)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
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

impl View for LabelConsumer {
    /// Records the visible label during render.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> leptatui::app::Result<()> {
        self.observed.borrow_mut().push(use_context::<ScopeLabel>());
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Component that provides a label during render and forwards events to a child.
struct EventLabelProvider {
    value: ScopeLabel,
    child: AnyView,
}

impl View for EventLabelProvider {
    /// Provides a label while rendering the provider subtree.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> leptatui::app::Result<()> {
        provide_context(self.value.clone());
        ctx.render_view(&self.child)
    }

    /// Forwards events to the child while this provider's scope is active.
    fn handle_event(&mut self, event: crossterm::event::Event) -> leptatui::app::Result<AppControl> {
        self.child.handle_event(event)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Component that records context visible during event handling.
struct EventLabelConsumer {
    observed: Rc<RefCell<Option<ScopeLabel>>>,
}

impl View for EventLabelConsumer {
    /// Renders nothing; this component only observes event-time context.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> leptatui::app::Result<()> {
        Ok(())
    }

    /// Records the label visible during event handling.
    fn handle_event(&mut self, _event: crossterm::event::Event) -> leptatui::app::Result<AppControl> {
        *self.observed.borrow_mut() = use_context::<ScopeLabel>();
        Ok(AppControl::Continue)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Component that provides active theme variables before rendering a child.
struct ThemeRenderRoot {
    dark: ReadSignal<bool>,
    child: AnyView,
    stylesheet: Stylesheet,
}

impl View for ThemeRenderRoot {
    /// Provides variables from the active boolean theme flag and renders the child.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context used to attach the stylesheet and render the child.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on successful child rendering.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> leptatui::app::Result<()> {
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
        ctx.__with_stylesheet(&self.stylesheet, |ctx| ctx.render_view(&self.child))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Component that provides active theme variables through a signal context.
struct ThemeSignalRoot {
    theme: ReadSignal<ThemeVariables>,
    child: AnyView,
    stylesheet: Stylesheet,
}

impl View for ThemeSignalRoot {
    /// Provides the theme signal and renders the child with stylesheet rules.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context used to attach the stylesheet and render the child.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on successful child rendering.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> leptatui::app::Result<()> {
        provide_context(self.theme);
        ctx.__with_stylesheet(&self.stylesheet, |ctx| ctx.render_view(&self.child))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Component that records context values observed during rendering.
struct ContextRoot {
    /// String context observed through Leptatui render-scope storage.
    observed_text: RefCell<Option<String>>,
    /// Signal value observed through Leptos owner fallback storage.
    observed_count: RefCell<Option<i32>>,
}

impl View for ContextRoot {
    /// Provides and reads context values during a render pass.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Rendering context supplied by the app root adapter.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> leptatui::app::Result<()> {
        provide_context(String::from("from component"));
        *self.observed_text.borrow_mut() = use_context::<String>();

        Owner::new().with(|| {
            let (count, set_count) = signal(1);
            leptos::context::provide_context(count);
            set_count.set(2);

            *self.observed_count.borrow_mut() =
                use_context::<ReadSignal<i32>>().map(|count| count.get_untracked());
        });

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
