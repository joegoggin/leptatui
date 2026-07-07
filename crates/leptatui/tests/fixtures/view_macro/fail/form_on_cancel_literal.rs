//! Fail fixture for literal `Form` cancel callbacks.

use leptatui::prelude::*;

/// Triggers the `on_cancel` callback validation failure.
fn main() {
    let _view: View = view! {
        <Form on_cancel="close">
            <Input value="Ada" />
        </Form>
    };
}
