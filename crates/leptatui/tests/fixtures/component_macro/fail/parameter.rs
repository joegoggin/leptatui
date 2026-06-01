use leptatui_macros::component;

#[component]
fn WithProps(label: String) -> leptatui::Node {
    leptatui::text(label)
}

fn main() {}
