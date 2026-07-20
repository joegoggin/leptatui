//! Pass fixture for controlled `TextArea` expansion.

use leptatui::prelude::*;

/// Exercises `TextArea` attributes and builder lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let value = String::from("Ada\nLovelace");
    let view = view! {
        <TextArea
            value=value
            placeholder="Notes"
            on_input={|_| AppControl::Continue}
            class="field primary"
            id="notes"
            style={style}
        />
    };

    assert_eq!(view.value(), "Ada\nLovelace");
    assert_eq!(view.placeholder_text(), Some("Notes"));
    assert_eq!(view.metadata().view_type(), ViewType::TextArea);
    assert_eq!(view.metadata().id(), Some("notes"));
    assert_eq!(
        view.metadata().classes(),
        &[String::from("field"), String::from("primary")]
    );
    assert_eq!(view.metadata().inline_style(), Some(style));
    assert!(view.has_on_input());
}
