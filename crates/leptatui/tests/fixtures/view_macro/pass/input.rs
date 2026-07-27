//! Pass fixture for controlled `Input` expansion.

use leptatui::prelude::*;

/// Exercises `Input` attributes and builder lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let value = String::from("Ada");
    let view = view! {
        <Input
            value=value
            placeholder="Name"
            on_input={|_| AppControl::Continue}
            class="field primary"
            id="name"
            style={style.clone()}
        />
    };

    assert_eq!(view.value(), "Ada");
    assert_eq!(view.placeholder_text(), Some("Name"));
    assert_eq!(view.metadata().view_type(), ViewType::Input);
    assert_eq!(view.metadata().id(), Some("name"));
    assert_eq!(
        view.metadata().classes(),
        &[String::from("field"), String::from("primary")]
    );
    assert_eq!(view.metadata().inline_style(), Some(style));
    assert!(view.has_on_input());
}
