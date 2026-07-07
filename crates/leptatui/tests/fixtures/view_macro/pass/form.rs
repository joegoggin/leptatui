//! Pass fixture for controlled `Form` expansion.

use leptatui::prelude::*;

/// Exercises `Form` attributes, callbacks, and editable child lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view: View = view! {
        <Form
            on_submit={|| AppControl::Continue}
            on_cancel={|| AppControl::Exit}
            class="form primary"
            id="profile"
            style={style}
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

    match view {
        View::Form {
            children,
            metadata,
            on_submit,
            on_cancel,
        } => {
            assert_eq!(children.len(), 2);
            assert_eq!(metadata.view_type(), ViewType::Form);
            assert_eq!(metadata.id(), Some("profile"));
            assert_eq!(
                metadata.classes(),
                &[String::from("form"), String::from("primary")]
            );
            assert_eq!(metadata.inline_style(), Some(style));
            assert!(on_submit.is_some());
            assert!(on_cancel.is_some());

            match &children[0] {
                View::Input {
                    value,
                    placeholder,
                    on_input,
                    ..
                } => {
                    assert_eq!(value, "Ada");
                    assert_eq!(placeholder.as_deref(), Some("Name"));
                    assert!(on_input.is_some());
                }
                other => panic!("expected input child, got {other:?}"),
            }

            match &children[1] {
                View::TextArea {
                    value,
                    placeholder,
                    on_input,
                    ..
                } => {
                    assert_eq!(value, "Notes");
                    assert_eq!(placeholder.as_deref(), Some("Notes"));
                    assert!(on_input.is_some());
                }
                other => panic!("expected text-area child, got {other:?}"),
            }
        }
        other => panic!("expected form view, got {other:?}"),
    }
}
