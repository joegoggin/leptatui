//! Pass fixture for Rust expression punctuation inside stylesheet values.

use leptatui::prelude::*;

/// Preserves block statements, match arms, closure arguments, and importance.
fn main() {
    let styles = stylesheet! {
        @mixin chrome {
            fg: {
                let colors = (Color::Yellow, Color::Blue);
                colors.0
            } !important;
        }
        @mixin inherited {
            @include chrome;
            bg: match true {
                true => Color::Black,
                false => Color::White,
            };
        }
        Text {
            @include inherited;
            padding: (|horizontal, vertical| TuiSpacing::new(horizontal, horizontal, vertical, vertical))(1, 2);
        }
    };
    let expected = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::Text),
        StyleDeclarations::new()
            .foreground_important(Color::Yellow)
            .background(Color::Black)
            .padding(TuiSpacing::new(1, 1, 2, 2)),
    );
    assert_eq!(styles, expected);
}
