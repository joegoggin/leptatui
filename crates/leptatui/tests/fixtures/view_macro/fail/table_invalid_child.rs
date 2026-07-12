//! Fail fixture for a row directly under a table.

use leptatui::prelude::*;

/// Triggers table-section child validation.
fn main() {
    let _: View = view! {
        <Table>
            <TableRow><TableCell>"Invalid"</TableCell></TableRow>
        </Table>
    };
}
