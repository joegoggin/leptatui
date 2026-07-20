//! Pass fixture for `ProgressBar` expansion.

use leptatui::prelude::*;

/// Exercises `ProgressBar` attributes and builder lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let progress = 0.5;
    let view = view! {
        <ProgressBar
            value=progress
            label="Uploading"
            class="meter primary"
            id="upload"
            style={style}
        />
    };

    assert_eq!(view.value(), progress);
    assert_eq!(view.label_text(), Some("Uploading"));
    assert_eq!(view.metadata().view_type(), ViewType::ProgressBar);
    assert_eq!(view.metadata().id(), Some("upload"));
    assert_eq!(
        view.metadata().classes(),
        &[String::from("meter"), String::from("primary")]
    );
    assert_eq!(view.metadata().inline_style(), Some(style));
}
