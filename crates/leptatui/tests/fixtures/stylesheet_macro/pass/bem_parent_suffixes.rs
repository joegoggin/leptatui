//! Pass fixture for nested BEM parent-suffix selectors.

use leptatui::prelude::*;

/// Exercises class suffix concatenation with ancestry, pseudo-classes, and media.
fn main() {
    let styles = stylesheet! {
        .shell => {
            .example-page => {
                &__button => {
                    &:focus => { bg: Color::Black }
                    &--submit => { fg: Color::Green }
                }
            }
        }

        .example-page => {
            &__actions => {
                @media (max-width: 60) {
                    flex_direction: FlexDirection::Column
                }
            }
        }
    };

    let expected = Stylesheet::new()
        .rule(
            StyleSelector::descendant(
                vec![StyleSelector::class("shell")],
                StyleSelector::compound(vec![
                    StyleSelector::class("example-page__button"),
                    StyleSelector::focus(),
                ]),
            ),
            TuiStyle::new().background(Color::Black),
        )
        .rule(
            StyleSelector::descendant(
                vec![StyleSelector::class("shell")],
                StyleSelector::class("example-page__button--submit"),
            ),
            TuiStyle::new().foreground(Color::Green),
        )
        .media_rule(
            MediaQuery::max_width(60),
            StyleSelector::class("example-page__actions"),
            TuiStyle::new().flex_direction(FlexDirection::Column),
        );

    assert_eq!(styles, expected);
}
