//! Pass fixture for nested layout views in `view!`.
//!
//! This binary verifies block and flex divs, text views, and buttons compose into the
//! expected render-tree shape.

use leptatui::prelude::*;

/// Exercises nested block and flex div expansion.
fn main() {
    let view = view! {
        <Div>
            <Text>{"Counter"}</Text>
            <Div style={TuiStyle::new().display(Display::Flex)}>
                <Button>"Increment"</Button>
                <Button>{"Reset"}</Button>
            </Div>
        </Div>
    };

    assert_eq!(
        view,
        div((
            text("Counter"),
            div([button("Increment"), button("Reset")])
                .with_inline_style(TuiStyle::new().display(Display::Flex)),
        ))
    );
}
