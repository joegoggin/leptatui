//! Pass fixture for `stylesheet!` media query blocks.

use leptatui::prelude::*;

/// Exercises top-level media blocks and nested selectors inside media blocks.
fn main() {
    let styles = stylesheet! {
        Text => { fg: Color::White }

        @media (max-width: 80) {
            .panel => {
                padding: TuiSpacing::ZERO,
                flex_direction: FlexDirection::Column

                Text => { fg: Color::Yellow }
            }

            Button:focus => { bg: Color::Blue }
        }

        @media (min-width: 81) and (min-height: 24) {
            #submit => { borders: Borders::NONE }
        }
    };

    let expected = Stylesheet::new()
        .rule(
            StyleSelector::view_type(ViewType::Text),
            TuiStyle::new().foreground(Color::White),
        )
        .media_rule(
            MediaQuery::max_width(80),
            StyleSelector::class("panel"),
            TuiStyle::new()
                .padding(TuiSpacing::ZERO)
                .flex_direction(FlexDirection::Column),
        )
        .media_rule(
            MediaQuery::max_width(80),
            StyleSelector::descendant(
                vec![StyleSelector::class("panel")],
                StyleSelector::view_type(ViewType::Text),
            ),
            TuiStyle::new().foreground(Color::Yellow),
        )
        .media_rule(
            MediaQuery::max_width(80),
            StyleSelector::compound(vec![
                StyleSelector::view_type(ViewType::Button),
                StyleSelector::focus(),
            ]),
            TuiStyle::new().background(Color::Blue),
        )
        .media_rule(
            MediaQuery::min_width(81).and(MediaQuery::min_height(24)),
            StyleSelector::id("submit"),
            TuiStyle::new().borders(Borders::NONE),
        );

    assert_eq!(styles, expected);
}
