//! Pass fixture for nested `stylesheet!` selectors.

use leptatui::prelude::*;

/// Exercises nested selector flattening into deterministic runtime selectors.
fn main() {
    let styles = stylesheet! {
        .panel => {
            bg: Color::Black

            Text => { fg: Color::White }

            Button => {
                &:focus => { background: Color::Yellow }
            }
        }
    };

    let expected = Stylesheet::new()
        .rule(
            StyleSelector::class("panel"),
            TuiStyle::new().background(Color::Black),
        )
        .rule(
            StyleSelector::descendant(
                vec![StyleSelector::class("panel")],
                StyleSelector::view_type(ViewType::Text),
            ),
            TuiStyle::new().foreground(Color::White),
        )
        .rule(
            StyleSelector::descendant(
                vec![StyleSelector::class("panel")],
                StyleSelector::compound(vec![
                    StyleSelector::view_type(ViewType::Button),
                    StyleSelector::focus(),
                ]),
            ),
            TuiStyle::new().background(Color::Yellow),
        );

    assert_eq!(styles, expected);
}
