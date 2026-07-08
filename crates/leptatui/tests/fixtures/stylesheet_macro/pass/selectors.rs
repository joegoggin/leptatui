//! Pass fixture for `stylesheet!` selector expansion.

use leptatui::prelude::*;

/// Exercises every supported `stylesheet!` selector shape.
fn main() {
    let styles = stylesheet! {
        Text => { fg: Color::White }
        Form => { fg: Color::Cyan }
        ProgressBar => { fg: Color::Green }
        .primary => { bg: Color::Blue, modifier: Modifier::BOLD }
        .primary-action => { fg: Color::Green }
        #submit => { padding: TuiSpacing::uniform(1) }
        #submit-button => { borders: Borders::ALL }
        :focus => { foreground: Color::Black }
        Button:focus => { background: Color::Yellow }
    };

    let expected = Stylesheet::new()
        .rule(
            StyleSelector::view_type(ViewType::Text),
            TuiStyle::new().foreground(Color::White),
        )
        .rule(
            StyleSelector::view_type(ViewType::Form),
            TuiStyle::new().foreground(Color::Cyan),
        )
        .rule(
            StyleSelector::view_type(ViewType::ProgressBar),
            TuiStyle::new().foreground(Color::Green),
        )
        .rule(
            StyleSelector::class("primary"),
            TuiStyle::new()
                .background(Color::Blue)
                .modifier(Modifier::BOLD),
        )
        .rule(
            StyleSelector::class("primary-action"),
            TuiStyle::new().foreground(Color::Green),
        )
        .rule(
            StyleSelector::id("submit"),
            TuiStyle::new().padding(TuiSpacing::uniform(1)),
        )
        .rule(
            StyleSelector::id("submit-button"),
            TuiStyle::new().borders(Borders::ALL),
        )
        .rule(
            StyleSelector::focus(),
            TuiStyle::new().foreground(Color::Black),
        )
        .rule(
            StyleSelector::compound(vec![
                StyleSelector::view_type(ViewType::Button),
                StyleSelector::focus(),
            ]),
            TuiStyle::new().background(Color::Yellow),
        );

    assert_eq!(styles, expected);

    let focused = button("Save").with_focus(true);
    let blurred = button("Cancel");
    let theme = ThemeVariables::default();
    let focused_style =
        styles.resolve(focused.style_metadata().unwrap(), &[], TuiStyle::new(), &theme);
    let blurred_style =
        styles.resolve(blurred.style_metadata().unwrap(), &[], TuiStyle::new(), &theme);

    assert_eq!(focused_style.background, Some(Color::Yellow));
    assert_ne!(blurred_style.background, Some(Color::Yellow));
}
