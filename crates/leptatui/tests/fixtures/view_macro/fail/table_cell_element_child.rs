//! Fail fixture for nested view content in a table cell.

use leptatui::prelude::*;

/// Triggers table-cell text-content validation.
fn main() {
    let _: View = view! {
        <Table>
            <TableBody>
                <TableRow><TableCell><Paragraph>"Nested"</Paragraph></TableCell></TableRow>
            </TableBody>
        </Table>
    };
}
