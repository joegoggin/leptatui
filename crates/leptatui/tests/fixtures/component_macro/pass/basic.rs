//! Pass fixture for basic `#[component]` expansion.
//!
//! This binary verifies generated component types satisfy the runtime component
//! contract for private and public component functions.

use leptatui::prelude::*;

/// Returns a node from a private component function.
#[component]
fn Greeting() -> Node {
    text("hello")
}

/// Returns borrowed text from a public component function.
#[component]
pub fn BorrowedText() -> &'static str {
    "borrowed"
}

/// Requires the generic type to implement [`Component`].
fn assert_component<T: Component>() {}

/// Exercises generated constructors and component trait implementations.
fn main() {
    assert_component::<Greeting>();
    assert_component::<BorrowedText>();

    let _private = Greeting::new();
    let _public = BorrowedText::default();
}
