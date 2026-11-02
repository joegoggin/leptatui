/// View that renders a required prop.
#[component]
fn MacroPropLabel(#[prop(into)] label: leptatui::Prop<String>) -> impl leptatui::IntoView {
    view! { <Text>{label.get()}</Text> }
}

/// View that renders a prop and nested children.
#[component]
fn MacroPropPanel(#[prop(into)] title: leptatui::Prop<String>, children: Children) -> impl leptatui::IntoView {
    view! {
        <Div>
            <Text>{title}</Text>
            {children()}
        </Div>
    }
}
