//! Pass fixture for media queries nested inside stylesheet rules.

use leptatui::prelude::*;

/// Exercises nested media declarations, selectors, mixins, BEM suffixes, and queries.
fn main() {
    let styles = stylesheet! {
        @mixin compact { padding: TuiSpacing::ZERO }

        .panel => {
            padding: TuiSpacing::uniform(1)

            @media (max-width: 80) {
                @include compact

                Text => { fg: Color::Yellow }
            }

            &__actions => {
                display: Display::Flex

                @media (max-width: 80) {
                    flex_direction: FlexDirection::Column
                }
            }
        }

        Button:focus => {
            @media (min-width: 81) and (min-height: 24) {
                bg: Color::Blue
            }
        }
    };

    let expected = Stylesheet::new()
        .rule(
            StyleSelector::class("panel"),
            TuiStyle::new().padding(TuiSpacing::uniform(1)),
        )
        .media_rule(
            MediaQuery::max_width(80),
            StyleSelector::class("panel"),
            TuiStyle::new().padding(TuiSpacing::ZERO),
        )
        .media_rule(
            MediaQuery::max_width(80),
            StyleSelector::descendant(
                vec![StyleSelector::class("panel")],
                StyleSelector::view_type(ViewType::Text),
            ),
            TuiStyle::new().foreground(Color::Yellow),
        )
        .rule(
            StyleSelector::class("panel__actions"),
            TuiStyle::new().display(Display::Flex),
        )
        .media_rule(
            MediaQuery::max_width(80),
            StyleSelector::class("panel__actions"),
            TuiStyle::new().flex_direction(FlexDirection::Column),
        )
        .media_rule(
            MediaQuery::min_width(81).and(MediaQuery::min_height(24)),
            StyleSelector::compound(vec![
                StyleSelector::view_type(ViewType::Button),
                StyleSelector::focus(),
            ]),
            TuiStyle::new().background(Color::Blue),
        );

    assert_eq!(styles, expected);
}
