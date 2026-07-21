/// Parent component with styled and plain sibling component subtrees.
#[component]
fn MacroSiblingStyleRoot() -> impl leptatui::IntoView {
    row([
        component(MacroStyledSibling::new()),
        component(MacroPlainSibling::new()),
    ])
}
