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

            A => {
                &:active => { foreground: Color::LightCyan }
            }

            Input => {
                &:insert => { foreground: Color::Yellow }
            }

            TextArea => {
                &:visual => { foreground: Color::Magenta }
            }

            Link => {
                &:visited => { foreground: Color::LightMagenta }
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
        )
        .rule(
            StyleSelector::descendant(
                vec![StyleSelector::class("panel")],
                StyleSelector::compound(vec![
                    StyleSelector::view_type(ViewType::A),
                    StyleSelector::active(),
                ]),
            ),
            TuiStyle::new().foreground(Color::LightCyan),
        )
        .rule(
            StyleSelector::descendant(
                vec![StyleSelector::class("panel")],
                StyleSelector::compound(vec![
                    StyleSelector::view_type(ViewType::Input),
                    StyleSelector::insert(),
                ]),
            ),
            TuiStyle::new().foreground(Color::Yellow),
        )
        .rule(
            StyleSelector::descendant(
                vec![StyleSelector::class("panel")],
                StyleSelector::compound(vec![
                    StyleSelector::view_type(ViewType::TextArea),
                    StyleSelector::visual(),
                ]),
            ),
            TuiStyle::new().foreground(Color::Magenta),
        )
        .rule(
            StyleSelector::descendant(
                vec![StyleSelector::class("panel")],
                StyleSelector::compound(vec![
                    StyleSelector::view_type(ViewType::Link),
                    StyleSelector::visited(),
                ]),
            ),
            TuiStyle::new().foreground(Color::LightMagenta),
        );

    assert_eq!(styles, expected);
}
