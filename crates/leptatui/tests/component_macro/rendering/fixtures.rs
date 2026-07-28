/// View whose internal layout changes height under a media rule.
#[component]
fn MacroResponsiveCaseRow() -> impl leptatui::IntoView {
    view! {
        <Div class="case-row">
            <Text>"type < class"</Text>
            <Text>"Sample"</Text>
        </Div>
    }
}

/// Parent component that must reserve the responsive child component height.
#[component]
fn MacroResponsiveCaseRoot() -> impl leptatui::IntoView {
    stylesheet! {
        .case-row => { display: Display::Flex }

        @media (max-width: 60) {
            .case-row => { flex_direction: FlexDirection::Column }
        }
    }

    view! {
        <Div>
            <Text>"Intro"</Text>
            <MacroResponsiveCaseRow />
        </Div>
    }
}

/// View with an overflowing internal layout.
#[component]
fn MacroScrollableList() -> impl leptatui::IntoView {
    div([
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
    div([component(MacroScrollableList::new())])
        .with_inline_style(TuiStyle::new().display(Display::Flex))
}

/// Bordered component root whose child content overflows vertically.
#[component]
fn MacroBorderedScrollableRoot() -> impl leptatui::IntoView {
    leptatui::block(div([
        text("One"),
        text("Two"),
        text("Three"),
        text("Four"),
        text("Five"),
        text("Six"),
    ]))
    .with_inline_style(
        TuiStyle::new().overflow(leptatui::Axes::new(
            leptatui::Overflow::Hidden,
            leptatui::Overflow::Auto,
        )),
    )
}

/// View containing the button scrolled out of a parent viewport.
#[component]
fn MacroMouseBoundaryButton() -> impl leptatui::IntoView {
    button("Hidden")
}

/// Parent view that replaces an off-screen component button visually.
#[component]
fn MacroScrolledMouseRoot() -> impl leptatui::IntoView {
    div((
        component(MacroMouseBoundaryButton::new()),
        button("Visible"),
    ))
}

/// Probe recording Markdown history traversal across component boundaries.
struct MacroMarkdownHistoryProbe {
    /// Last requested history direction, where `true` means backward.
    direction: Rc<Cell<Option<bool>>>,
}

impl View for MacroMarkdownHistoryProbe {
    /// Renders the behavior-only probe without terminal output.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> leptatui::app::Result<()> {
        Ok(())
    }

    /// Returns the probe for concrete-type inspection.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Returns the mutable probe for concrete-type inspection.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Records a Markdown history traversal request.
    fn __navigate_markdown_history(&mut self, back: bool) -> bool {
        self.direction.set(Some(back));
        true
    }
}

/// Generated component forwarding Markdown history to its stored view.
#[component]
fn MacroMarkdownHistoryBoundary(probe: AnyView) -> impl leptatui::IntoView {
    probe
}
