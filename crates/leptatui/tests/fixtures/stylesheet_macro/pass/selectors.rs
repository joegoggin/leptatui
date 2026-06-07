//! Pass fixture for `stylesheet!` selector expansion.

use leptatui::prelude::*;

/// Exercises every supported `stylesheet!` selector shape.
fn main() {
    let styles = stylesheet! {
        Text => { fg: Color::White }
        .primary => { bg: Color::Blue, modifier: Modifier::BOLD }
        #submit => { padding: TuiSpacing::uniform(1) }
        :focus => { foreground: Color::Black }
        Button:focus => { background: Color::Yellow }
    };

    let expected = Stylesheet::new()
        .rule(
            StyleSelector::node_type(NodeType::Text),
            TuiStyle::new().foreground(Color::White),
        )
        .rule(
            StyleSelector::class("primary"),
            TuiStyle::new()
                .background(Color::Blue)
                .modifier(Modifier::BOLD),
        )
        .rule(
            StyleSelector::id("submit"),
            TuiStyle::new().padding(TuiSpacing::uniform(1)),
        )
        .rule(
            StyleSelector::focus(),
            TuiStyle::new().foreground(Color::Black),
        )
        .rule(
            StyleSelector::compound(vec![
                StyleSelector::node_type(NodeType::Button),
                StyleSelector::focus(),
            ]),
            TuiStyle::new().background(Color::Yellow),
        );

    assert_eq!(styles, expected);

    let focused = button("Save").with_focus(true);
    let blurred = button("Cancel");
    let focused_style = styles.resolve(focused.style_metadata().unwrap(), TuiStyle::new());
    let blurred_style = styles.resolve(blurred.style_metadata().unwrap(), TuiStyle::new());

    assert_eq!(focused_style.background, Some(Color::Yellow));
    assert_ne!(blurred_style.background, Some(Color::Yellow));
}
