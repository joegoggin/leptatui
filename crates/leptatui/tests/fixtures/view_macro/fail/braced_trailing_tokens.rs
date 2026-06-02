use leptatui::prelude::*;

fn main() {
    let label = String::from("bad");

    let _ = view! {
        <Text>{label extra}</Text>
    };
}
