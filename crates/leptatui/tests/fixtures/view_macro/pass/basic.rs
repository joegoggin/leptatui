//! Pass fixture for basic `view!` expansion.
//!
//! This binary verifies supported element attributes populate selector metadata
//! while nested elements lower into the expected view tree.

use leptatui::prelude::*;

/// Exercises basic element expansion and accepted attributes.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view: View = view! {
        <Block class="card active" id="main" style={style}>
            <Text class={"label"}>"hello"</Text>
        </Block>
    };

    match view {
        View::Block { child, metadata } => {
            assert_eq!(metadata.view_type(), ViewType::Block);
            assert_eq!(metadata.id(), Some("main"));
            assert_eq!(
                metadata.classes(),
                &[String::from("card"), String::from("active")]
            );
            assert_eq!(metadata.inline_style(), Some(style));

            match *child {
                View::Text { content, metadata } => {
                    assert_eq!(content, "hello");
                    assert_eq!(metadata.view_type(), ViewType::Text);
                    assert_eq!(metadata.classes(), &[String::from("label")]);
                }
                other => panic!("expected text child, got {other:?}"),
            }
        }
        other => panic!("expected block view, got {other:?}"),
    }

    let action_view: View = view! {
        <Button on_press={|| AppControl::Continue}>"Save"</Button>
    };

    match action_view {
        View::Button { on_press, .. } => assert!(on_press.is_some()),
        other => panic!("expected button view, got {other:?}"),
    }

    let presses = std::rc::Rc::new(std::cell::Cell::new(0));
    let presses_for_button = std::rc::Rc::clone(&presses);
    let move_action_view: View = view! {
        <Button
            on_press=move || {
                presses_for_button.set(presses_for_button.get() + 1);
                AppControl::Continue
            }
        >
            "Increment"
        </Button>
    };

    match move_action_view {
        View::Button { on_press, .. } => assert!(on_press.is_some()),
        other => panic!("expected button view, got {other:?}"),
    }

    let shorthand_action_view: View = view! {
        <Button on_press=|| AppControl::Exit>"Quit"</Button>
    };

    match shorthand_action_view {
        View::Button { on_press, .. } => assert!(on_press.is_some()),
        other => panic!("expected button view, got {other:?}"),
    }
}
