use leptatui::{BorderType, Borders, Color, Modifier, TuiSpacing, TuiStyle};
use ratatui::{style::Style, widgets::Padding};

#[test]
fn tui_style_maps_to_ratatui_style() {
    let style = TuiStyle::new()
        .foreground(Color::Yellow)
        .background(Color::Black)
        .modifier(Modifier::BOLD | Modifier::ITALIC);

    assert_eq!(
        style.to_ratatui_style(),
        Style::new()
            .fg(Color::Yellow)
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD | Modifier::ITALIC)
    );
}

#[test]
fn tui_spacing_maps_to_ratatui_padding() {
    assert_eq!(
        Padding::from(TuiSpacing::new(1, 2, 3, 4)),
        Padding::new(1, 2, 3, 4)
    );
}

#[test]
fn tui_style_builds_a_block_with_border_configuration() {
    let style = TuiStyle::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(TuiSpacing::uniform(1));

    let _block = style.to_block();
}
