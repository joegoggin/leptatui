//! Pass fixture for `ProgressBar` expansion.

use leptatui::prelude::*;

/// Exercises `ProgressBar` attributes and builder lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let progress = 0.5;
    let view: View = view! {
        <ProgressBar
            value=progress
            label="Uploading"
            class="meter primary"
            id="upload"
            style={style}
        />
    };

    match view {
        View::ProgressBar {
            value,
            label,
            metadata,
        } => {
            assert_eq!(value, progress);
            assert_eq!(label.as_deref(), Some("Uploading"));
            assert_eq!(metadata.view_type(), ViewType::ProgressBar);
            assert_eq!(metadata.id(), Some("upload"));
            assert_eq!(
                metadata.classes(),
                &[String::from("meter"), String::from("primary")]
            );
            assert_eq!(metadata.inline_style(), Some(style));
        }
        other => panic!("expected progress bar view, got {other:?}"),
    }
}
