//! Style conversion tests.
//!
//! These tests cover conversion from Leptatui style helpers into Ratatui style,
//! padding, and block values.

use leptatui::{
    BorderType, Borders, Color, Modifier, NodeType, StyleSelector, Stylesheet, TuiSpacing,
    TuiStyle, button, text,
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

/// Verifies class stylesheet rules override type stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// text("Save").with_classes("primary")
/// Stylesheet::new()
///     .rule(StyleSelector::node_type(NodeType::Text), white)
///     .rule(StyleSelector::class("primary"), yellow)
/// ```
///
/// # Assertions
///
/// - Node metadata is available for stylesheet resolution.
/// - The resolved foreground color is yellow.
///
/// # Why
///
/// Class selectors should have higher specificity than type selectors.
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

/// Verifies id stylesheet rules override class stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// text("Save").with_classes("primary").with_id("save")
/// Stylesheet::new()
///     .rule(StyleSelector::class("primary"), yellow)
///     .rule(StyleSelector::id("save"), green)
/// ```
///
/// # Assertions
///
/// - Node metadata is available for stylesheet resolution.
/// - The resolved foreground color is green.
///
/// # Why
///
/// Id selectors should have higher specificity than class selectors.
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

/// Verifies inline styles override stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// text("Save").with_id("save").with_inline_style(black)
/// Stylesheet::new().rule(StyleSelector::id("save"), green)
/// ```
///
/// # Assertions
///
/// - Node metadata is available for stylesheet resolution.
/// - The resolved foreground color is black.
///
/// # Why
///
/// Inline styles are the final override in style resolution.
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

/// Verifies inherited colors remain unless the node overrides them.
///
/// # Example Under Test
///
/// ```text
/// text("Child").with_inline_style(foreground: yellow)
/// inherited = foreground: green, background: blue
/// ```
///
/// # Assertions
///
/// - Node metadata is available for stylesheet resolution.
/// - The resolved foreground color is yellow.
/// - The resolved background color is blue.
///
/// # Why
///
/// Child styles should preserve inherited fields that are not locally set.
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

/// Verifies focus selectors match only focused nodes.
///
/// # Example Under Test
///
/// ```text
/// button("Save").with_focus(true)
/// button("Cancel")
/// Stylesheet::new().rule(StyleSelector::focus(), yellow)
/// ```
///
/// # Assertions
///
/// - Focused button metadata is available for stylesheet resolution.
/// - Blurred button metadata is available for stylesheet resolution.
/// - The focused button resolves to a yellow foreground.
/// - The blurred button resolves with no foreground color.
///
/// # Why
///
/// Focus styling should depend on node focus state, not just node type.
#[test]
fn stylesheet_focus_selector_matches_only_focused_nodes() {
    let focused = button("Save").with_focus(true);
    let blurred = button("Cancel");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::focus(),
        TuiStyle::new().foreground(Color::Yellow),
    );

    let focused_style = stylesheet.resolve(focused.style_metadata().unwrap(), TuiStyle::new());
    let blurred_style = stylesheet.resolve(blurred.style_metadata().unwrap(), TuiStyle::new());

    assert_eq!(focused_style.foreground, Some(Color::Yellow));
    assert_eq!(blurred_style.foreground, None);
}
