//! Fail fixture for literal `Form` submit callbacks.

use leptatui::prelude::*;

/// Triggers the `on_submit` callback validation failure.
fn main() {
    let _view = view! {
        <Form on_submit="save">
            <Input value="Ada" />
        </Form>
    };
}
