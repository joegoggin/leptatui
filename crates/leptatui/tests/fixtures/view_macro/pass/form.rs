//! Pass fixture for `Form` expansion.

use leptatui::prelude::*;

/// Exercises `Form` attributes, callbacks, and child lowering.
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
            <Input value="Ada" />
        </Form>
    };

    match view {
        View::Form {
            children,
            metadata,
            on_submit,
            on_cancel,
        } => {
            assert_eq!(children.len(), 1);
            assert_eq!(metadata.view_type(), ViewType::Form);
            assert_eq!(metadata.id(), Some("profile"));
            assert_eq!(
                metadata.classes(),
                &[String::from("form"), String::from("primary")]
            );
            assert_eq!(metadata.inline_style(), Some(style));
            assert!(on_submit.is_some());
            assert!(on_cancel.is_some());
        }
        other => panic!("expected form view, got {other:?}"),
    }
}
