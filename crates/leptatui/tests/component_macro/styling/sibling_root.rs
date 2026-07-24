/// Parent component with styled and plain sibling component subtrees.
#[component]
fn MacroSiblingStyleRoot() -> impl leptatui::IntoView {
    div([
        component(MacroStyledSibling::new()),
        component(MacroPlainSibling::new()),
    ])
    .with_inline_style(TuiStyle::new().display(Display::Flex))
}
