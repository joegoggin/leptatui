//! Pass fixture for controlled `TextArea` expansion.

use leptatui::prelude::*;

/// Exercises `TextArea` attributes and builder lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view: View = view! {
        <TextArea
            value={"Ada\nLovelace".to_string()}
            placeholder="Notes"
            on_input={|_| AppControl::Continue}
            class="field primary"
            id="notes"
            style={style}
        />
    };

    match view {
        View::TextArea {
            value,
            placeholder,
            metadata,
            on_input,
            ..
        } => {
            assert_eq!(value, "Ada\nLovelace");
            assert_eq!(placeholder.as_deref(), Some("Notes"));
            assert_eq!(metadata.view_type(), ViewType::TextArea);
            assert_eq!(metadata.id(), Some("notes"));
            assert_eq!(
                metadata.classes(),
                &[String::from("field"), String::from("primary")]
            );
            assert_eq!(metadata.inline_style(), Some(style));
            assert!(on_input.is_some());
        }
        other => panic!("expected text-area view, got {other:?}"),
    }
}
