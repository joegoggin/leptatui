//! Pass fixture for `stylesheet!` selector expansion.

use leptatui::prelude::*;

/// Exercises every supported `stylesheet!` selector shape.
fn main() {
    let styles = stylesheet! {
        Text => { fg: Color::White }
        Form => { fg: Color::Cyan }
        Input => { fg: Color::LightBlue }
        TextArea => { fg: Color::LightMagenta }
        Image => { fg: Color::Yellow }
        ProgressBar => { fg: Color::Green }
        .primary => { bg: Color::Blue, modifier: Modifier::BOLD }
        .primary-action => { fg: Color::Green }
        #submit => { padding: TuiSpacing::uniform(1) }
        #submit-button => { borders: Borders::ALL }
        :focus => { foreground: Color::Black }
        :active => { foreground: Color::LightCyan }
        :insert => { foreground: Color::Yellow }
        :visual => { foreground: Color::Magenta }
        :visited => { foreground: Color::LightMagenta }
        Button:focus => { background: Color::Yellow }
        A:active => { modifier: Modifier::BOLD }
        Input:insert => { background: Color::White }
        TextArea:visual => { background: Color::Black }
        Link:visited => { modifier: Modifier::UNDERLINED }
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
            StyleSelector::view_type(ViewType::Input),
            TuiStyle::new().foreground(Color::LightBlue),
        )
        .rule(
            StyleSelector::view_type(ViewType::TextArea),
            TuiStyle::new().foreground(Color::LightMagenta),
        )
        .rule(
            StyleSelector::view_type(ViewType::Image),
            TuiStyle::new().foreground(Color::Yellow),
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
            StyleSelector::active(),
            TuiStyle::new().foreground(Color::LightCyan),
        )
        .rule(
            StyleSelector::insert(),
            TuiStyle::new().foreground(Color::Yellow),
        )
        .rule(
            StyleSelector::visual(),
            TuiStyle::new().foreground(Color::Magenta),
        )
        .rule(
            StyleSelector::visited(),
            TuiStyle::new().foreground(Color::LightMagenta),
        )
        .rule(
            StyleSelector::compound(vec![
                StyleSelector::view_type(ViewType::Button),
                StyleSelector::focus(),
            ]),
            TuiStyle::new().background(Color::Yellow),
        )
        .rule(
            StyleSelector::compound(vec![
                StyleSelector::view_type(ViewType::A),
                StyleSelector::active(),
            ]),
            TuiStyle::new().modifier(Modifier::BOLD),
        )
        .rule(
            StyleSelector::compound(vec![
                StyleSelector::view_type(ViewType::Input),
                StyleSelector::insert(),
            ]),
            TuiStyle::new().background(Color::White),
        )
        .rule(
            StyleSelector::compound(vec![
                StyleSelector::view_type(ViewType::TextArea),
                StyleSelector::visual(),
            ]),
            TuiStyle::new().background(Color::Black),
        )
        .rule(
            StyleSelector::compound(vec![
                StyleSelector::view_type(ViewType::Link),
                StyleSelector::visited(),
            ]),
            TuiStyle::new().modifier(Modifier::UNDERLINED),
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

    let active = AnyView::new(text("Docs")).with_active(true);
    let inactive = AnyView::new(text("Home"));
    let active_style = styles.resolve(
        active.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &theme,
    );
    let inactive_style = styles.resolve(
        inactive.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &theme,
    );

    assert_eq!(active_style.foreground, Some(Color::LightCyan));
    assert_ne!(inactive_style.foreground, Some(Color::LightCyan));

    let insert = AnyView::new(text("Editing")).with_insert(true);
    let insert_style = styles.resolve(
        insert.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &theme,
    );
    assert_eq!(insert_style.foreground, Some(Color::Yellow));

    let visual = AnyView::new(text("Selecting")).with_visual(true);
    let visual_style = styles.resolve(
        visual.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &theme,
    );
    assert_eq!(visual_style.foreground, Some(Color::Magenta));

    let visited = AnyView::new(text("Opened")).with_visited(true);
    let visited_style = styles.resolve(
        visited.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &theme,
    );
    assert_eq!(visited_style.foreground, Some(Color::LightMagenta));
}
