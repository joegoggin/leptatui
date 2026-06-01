use leptatui::{Component, text};
use leptatui_macros::component;

#[component]
fn Greeting() -> leptatui::Node {
    text("hello")
}

#[component]
pub fn BorrowedText() -> &'static str {
    "borrowed"
}

fn assert_component<T: Component>() {}

fn main() {
    assert_component::<Greeting>();
    assert_component::<BorrowedText>();

    let _private = Greeting::new();
    let _public = BorrowedText::default();
}
