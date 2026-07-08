//! Pass fixture for `stylesheet!` variables.

use leptatui::prelude::*;

/// Defines a stylesheet that reuses variables across rules.
fn main() {
    let _styles = stylesheet! {
        $primary: Color::LightCyan;
        $surface: Color::Black;
        $pad: TuiSpacing::horizontal(2);
        $mobile_direction: LayoutDirection::Column;
        $thumbnail: TuiSize::new(24, 8);

        Text => { fg: $primary, bg: $surface }
        .panel => {
            padding: $pad,
            direction: $mobile_direction
        }
        .thumbnail => { image_size: $thumbnail }
    };
}
