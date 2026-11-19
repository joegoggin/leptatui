//! Rejects input notifications with a value argument.

use leptatui::prelude::*;

/// Exercises the zero-argument notification signature.
fn main() {
    let value = RwSignal::new(String::from("Ada"));
    let _ = view! { <TextArea value=value on_input=|_value: String| {} /> };
}
