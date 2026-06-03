//! Style conversion tests.
//!
//! These tests cover conversion from Leptatui style helpers into Ratatui style,
//! padding, and block values.

use leptatui::{
    text, BorderType, Borders, Color, Modifier, NodeType, StyleSelector, Stylesheet, TuiSpacing,
    TuiStyle,
};
use ratatui::{style::Style, widgets::Padding};

/// Verifies terminal UI style maps to Ratatui style.
///
/// # Example Under Test
///
/// ```text
/// TuiStyle::new()
///     .foreground(Color::Yellow)
///     .background(Color::Black)
///     .modifier(Modifier::BOLD | Modifier::ITALIC)
/// ```
///
/// # Assertions
///
/// - The converted style has a yellow foreground.
/// - The converted style has a black background.
/// - The converted style has bold and italic modifiers.
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

/// Verifies terminal UI spacing maps to Ratatui padding.
///
/// # Example Under Test
///
/// ```text
/// TuiSpacing::new(1, 2, 3, 4)
/// ```
///
/// # Assertions
///
/// - Left padding is `1`.
/// - Right padding is `2`.
/// - Top padding is `3`.
/// - Bottom padding is `4`.
#[test]
fn tui_spacing_maps_to_ratatui_padding() {
    assert_eq!(
        Padding::from(TuiSpacing::new(1, 2, 3, 4)),
        Padding::new(1, 2, 3, 4)
    );
}

/// Verifies terminal UI style can build a configured Ratatui block.
///
/// # Example Under Test
///
/// ```text
/// TuiStyle::new()
///     .borders(Borders::ALL)
///     .border_type(BorderType::Rounded)
///     .padding(TuiSpacing::uniform(1))
/// ```
///
/// # Assertions
///
/// - A block can be built from a style with borders.
/// - A block can be built from a style with rounded border glyphs.
/// - A block can be built from a style with uniform padding.
#[test]
fn tui_style_builds_a_block_with_border_configuration() {
    let style = TuiStyle::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(TuiSpacing::uniform(1));

    let _block = style.to_block();
}

#[test]
fn stylesheet_class_overrides_type_style() {
    let node = text("Save").with_classes("primary");
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::node_type(NodeType::Text),
            TuiStyle::new().foreground(Color::White),
        )
        .rule(
            StyleSelector::class("primary"),
            TuiStyle::new().foreground(Color::Yellow),
        );

    let resolved = stylesheet.resolve(node.style_metadata().unwrap(), TuiStyle::new());

    assert_eq!(resolved.foreground, Some(Color::Yellow));
}

#[test]
fn stylesheet_id_overrides_class_style() {
    let node = text("Save").with_classes("primary").with_id("save");
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::class("primary"),
            TuiStyle::new().foreground(Color::Yellow),
        )
        .rule(
            StyleSelector::id("save"),
            TuiStyle::new().foreground(Color::Green),
        );

    let resolved = stylesheet.resolve(node.style_metadata().unwrap(), TuiStyle::new());

    assert_eq!(resolved.foreground, Some(Color::Green));
}

#[test]
fn inline_style_overrides_stylesheet_rules() {
    let node = text("Save")
        .with_id("save")
        .with_inline_style(TuiStyle::new().foreground(Color::Black));
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::id("save"),
        TuiStyle::new().foreground(Color::Green),
    );

    let resolved = stylesheet.resolve(node.style_metadata().unwrap(), TuiStyle::new());

    assert_eq!(resolved.foreground, Some(Color::Black));
}

#[test]
fn inherited_colors_flow_to_children_unless_overridden() {
    let node = text("Child").with_inline_style(TuiStyle::new().foreground(Color::Yellow));
    let inherited = TuiStyle::new()
        .foreground(Color::Green)
        .background(Color::Blue);

    let resolved = Stylesheet::new().resolve(node.style_metadata().unwrap(), inherited);

    assert_eq!(resolved.foreground, Some(Color::Yellow));
    assert_eq!(resolved.background, Some(Color::Blue));
}
