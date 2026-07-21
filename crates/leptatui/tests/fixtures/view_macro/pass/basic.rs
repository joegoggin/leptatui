//! Pass fixture for basic `view!` expansion.
//!
//! This binary verifies supported element attributes populate selector metadata
//! while nested elements lower into the expected view tree.

use leptatui::prelude::*;

/// Exercises basic element expansion and accepted attributes.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view = view! {
        <Block class="card active" id="main" style={style}>
            <Text class={"label"}>"hello"</Text>
        </Block>
    };

    assert_eq!(view.metadata().view_type(), ViewType::Block);
    assert_eq!(view.metadata().id(), Some("main"));
    assert_eq!(
        view.metadata().classes(),
        &[String::from("card"), String::from("active")]
    );
    assert_eq!(view.metadata().inline_style(), Some(style));
    let child = view.children()[0]
        .downcast_ref::<TextView>()
        .expect("expected text child");
    assert_eq!(child.content().to_string(), "hello");
    assert_eq!(child.metadata().view_type(), ViewType::Text);
    assert_eq!(child.metadata().classes(), &[String::from("label")]);

    let unbraced_style_view = view! {
        <Text style=TuiStyle::new().foreground(Color::LightCyan)>"inline"</Text>
    };

    assert_eq!(
        unbraced_style_view.metadata().inline_style(),
        Some(TuiStyle::new().foreground(Color::LightCyan))
    );

    let action_view = view! {
        <Button on_press={|| AppControl::Continue}>"Save"</Button>
    };

    assert!(action_view.has_on_press());

    let presses = std::rc::Rc::new(std::cell::Cell::new(0));
    let presses_for_button = std::rc::Rc::clone(&presses);
    let move_action_view = view! {
        <Button
            on_press=move || {
                presses_for_button.set(presses_for_button.get() + 1);
                AppControl::Continue
            }
        >
            "Increment"
        </Button>
    };

    assert!(move_action_view.has_on_press());

    let shorthand_action_view = view! {
        <Button on_press=|| AppControl::Exit>"Quit"</Button>
    };

    assert!(shorthand_action_view.has_on_press());
}
