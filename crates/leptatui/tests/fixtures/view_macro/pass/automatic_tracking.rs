//! Pass fixture for implicit tracking and owned captures.

use leptatui::prelude::*;

include!("../inspect.rs");

/// Renders changing data supplied through the generated prop builder.
///
/// # Arguments
///
/// * `label` — Current display label.
///
/// # Returns
///
/// A text view that reads the current prop.
#[component]
fn AutomaticLabel(label: Prop<String>) -> impl IntoView {
    view! { <Text>{label.get()}</Text> }
}

/// Exercises ordinary expressions, owned strings, and locally bound variables.
fn main() {
    Owner::new().with(|| {
        let label = String::from("owned");
        let values = RwSignal::new(vec![String::from("First")]);
        let title = RwSignal::new(String::from("Initial"));
        let view = view! {
            <Div>
                <Text>{format!("{label}")}</Text>
                <Text>{label}</Text>
                <Input value={title.get()} placeholder={label} />
                <AutomaticLabel label={title.get()} />
                {values.get().into_iter().map(|label| {
                    let local = format!("Row: {label}");
                    text(local)
                }).collect::<Vec<_>>()}
            </Div>
        };
        assert_eq!(label, "owned");
        inspect::<InputView>(&view, |node| assert_eq!(node.value(), "Initial"));
        title.set(String::from("Changed"));
        inspect::<InputView>(&view, |node| assert_eq!(node.value(), "Changed"));
    });
}
