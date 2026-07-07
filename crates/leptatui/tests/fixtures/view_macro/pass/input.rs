//! Pass fixture for controlled `Input` expansion.

use leptatui::prelude::*;

/// Exercises `Input` attributes and builder lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let value = String::from("Ada");
    let view: View = view! {
        <Input
            value=value
            placeholder="Name"
            on_input={|_| AppControl::Continue}
            class="field primary"
            id="name"
            style={style}
        />
    };

    match view {
        View::Input {
            value,
            placeholder,
            metadata,
            on_input,
            ..
        } => {
            assert_eq!(value, "Ada");
            assert_eq!(placeholder.as_deref(), Some("Name"));
            assert_eq!(metadata.view_type(), ViewType::Input);
            assert_eq!(metadata.id(), Some("name"));
            assert_eq!(
                metadata.classes(),
                &[String::from("field"), String::from("primary")]
            );
            assert_eq!(metadata.inline_style(), Some(style));
            assert!(on_input.is_some());
        }
        other => panic!("expected input view, got {other:?}"),
    }
}
