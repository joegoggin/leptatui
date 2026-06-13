//! Pass fixture for `stylesheet!` reusable mixins.

use leptatui::prelude::*;

/// Exercises mixin declaration, reuse, and rule-local overrides.
fn main() {
    let styles = stylesheet! {
        @mixin panel_chrome {
            bg: Color::Black,
            padding: TuiSpacing::uniform(1)
        }

        @mixin control_chrome {
            fg: Color::White,
            bg: Color::Blue
        }

        .panel => { @include panel_chrome }
        Button => { @include control_chrome, fg: Color::Yellow }
        .primary => { @include control_chrome }
    };

    let expected = Stylesheet::new()
        .rule(
            StyleSelector::class("panel"),
            TuiStyle::new()
                .background(Color::Black)
                .padding(TuiSpacing::uniform(1)),
        )
        .rule(
            StyleSelector::view_type(ViewType::Button),
            TuiStyle::new()
                .foreground(Color::White)
                .background(Color::Blue)
                .foreground(Color::Yellow),
        )
        .rule(
            StyleSelector::class("primary"),
            TuiStyle::new()
                .foreground(Color::White)
                .background(Color::Blue),
        );

    assert_eq!(styles, expected);
}
