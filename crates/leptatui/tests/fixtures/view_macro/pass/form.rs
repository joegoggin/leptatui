//! Pass fixture for controlled `Form` expansion.

use leptatui::prelude::*;

/// Exercises `Form` attributes, callbacks, and editable child lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view = view! {
        <Form
            on_submit={|| AppControl::Continue}
            on_cancel={|| AppControl::Exit}
            class="form primary"
            id="profile"
            style={style.clone()}
        >
            <Input
                value="Ada"
                placeholder="Name"
                on_input={|_| AppControl::Continue}
            />
            <TextArea
                value={"Notes".to_string()}
                placeholder="Notes"
                on_input={|_| AppControl::Continue}
            />
        </Form>
    };

    assert_eq!(view.children().len(), 2);
    assert_eq!(view.metadata().view_type(), ViewType::Form);
    assert_eq!(view.metadata().id(), Some("profile"));
    assert_eq!(
        view.metadata().classes(),
        &[String::from("form"), String::from("primary")]
    );
    assert_eq!(view.metadata().inline_style(), Some(style));
    assert!(view.has_on_submit());
    assert!(view.has_on_cancel());

    let input = view.children()[0]
        .downcast_ref::<InputView>()
        .expect("expected input child");
    assert_eq!(input.value(), "Ada");
    assert_eq!(input.placeholder_text(), Some("Name"));
    assert!(input.has_on_input());

    let text_area = view.children()[1]
        .downcast_ref::<TextAreaView>()
        .expect("expected text-area child");
    assert_eq!(text_area.value(), "Notes");
    assert_eq!(text_area.placeholder_text(), Some("Notes"));
    assert!(text_area.has_on_input());
}
