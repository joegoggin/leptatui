//! Fail fixture for a cell directly under a table body.

use leptatui::prelude::*;

/// Triggers table-section row validation.
fn main() {
    let _ = view! {
        <Table>
            <TableBody><TableCell>"Invalid"</TableCell></TableBody>
        </Table>
    };
}
