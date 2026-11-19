//! Rejects string-valued editable controls.

use leptatui::prelude::*;

/// Exercises the requirement to pass the signal rather than its text.
fn main() {
    let value = RwSignal::new(String::from("Ada"));
    let _ = view! { <TextArea value={value.get()} /> };
}
