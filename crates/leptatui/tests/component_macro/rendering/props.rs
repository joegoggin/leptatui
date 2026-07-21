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
