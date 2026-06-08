//! Style conversion tests.
//!
//! These tests cover conversion from Leptatui style helpers into Ratatui style,
//! padding, and block values.

use leptatui::{
    BorderType, Borders, Color, Modifier, NodeType, StyleMetadata, StyleSelector, Stylesheet,
    TuiSpacing, TuiStyle, button, stylesheet, text,
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

    let resolved = stylesheet.resolve(node.style_metadata().unwrap(), &[], TuiStyle::new());

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

    let resolved = stylesheet.resolve(node.style_metadata().unwrap(), &[], TuiStyle::new());

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

    let resolved = stylesheet.resolve(node.style_metadata().unwrap(), &[], TuiStyle::new());

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

    let resolved = Stylesheet::new().resolve(node.style_metadata().unwrap(), &[], inherited);

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

    let focused_style = stylesheet.resolve(focused.style_metadata().unwrap(), &[], TuiStyle::new());
    let blurred_style = stylesheet.resolve(blurred.style_metadata().unwrap(), &[], TuiStyle::new());

    assert_eq!(focused_style.foreground, Some(Color::Yellow));
    assert_eq!(blurred_style.foreground, None);
}

/// Verifies descendant selectors match ancestor metadata in source order.
///
/// # Example Under Test
///
/// ```text
/// ancestors = [.app, .panel]
/// target = Button
/// selector = descendant([.app, .panel], Button)
/// ```
///
/// # Assertions
///
/// - The button metadata is available for stylesheet resolution.
/// - The selector matches when `.app` appears before `.panel`.
/// - The selector does not match when `.panel` appears before `.app`.
///
/// # Why
///
/// Descendant selector matching should honor ordered render ancestors without
/// requiring direct parent-child adjacency.
#[test]
fn descendant_selector_matches_ordered_ancestors() {
    let mut app = StyleMetadata::new(NodeType::Column);
    app.set_classes("app");
    let mut panel = StyleMetadata::new(NodeType::Block);
    panel.set_classes("panel");
    let button = button("Save");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::descendant(
            vec![StyleSelector::class("app"), StyleSelector::class("panel")],
            StyleSelector::node_type(NodeType::Button),
        ),
        TuiStyle::new().foreground(Color::Yellow),
    );

    let matched = stylesheet.resolve(
        button.style_metadata().unwrap(),
        &[app.clone(), panel.clone()],
        TuiStyle::new(),
    );
    let wrong_order = stylesheet.resolve(
        button.style_metadata().unwrap(),
        &[panel, app],
        TuiStyle::new(),
    );

    assert_eq!(matched.foreground, Some(Color::Yellow));
    assert_eq!(wrong_order.foreground, None);
}

/// Verifies nested stylesheet macro rules resolve against ancestor metadata.
///
/// # Example Under Test
///
/// ```text
/// .panel => {
///     Button => {
///         &:focus => { fg: Color::Yellow }
///     }
/// }
/// ```
///
/// # Assertions
///
/// - The macro accepts nested rules with `&:focus`.
/// - The focused button resolves to a yellow foreground under `.panel`.
/// - The blurred button resolves with no foreground color under `.panel`.
///
/// # Why
///
/// Nested macro selectors should lower into descendant selectors that preserve
/// terminal-node focus matching.
#[test]
fn stylesheet_macro_nested_selectors_resolve_against_ancestors() {
    let styles = stylesheet! {
        .panel => {
            Button => {
                &:focus => { fg: Color::Yellow }
            }
        }
    };
    let mut panel = StyleMetadata::new(NodeType::Block);
    panel.set_classes("panel");
    let focused = button("Save").with_focus(true);
    let blurred = button("Cancel");

    let focused_style = styles.resolve(
        focused.style_metadata().unwrap(),
        &[panel.clone()],
        TuiStyle::new(),
    );
    let blurred_style =
        styles.resolve(blurred.style_metadata().unwrap(), &[panel], TuiStyle::new());

    assert_eq!(focused_style.foreground, Some(Color::Yellow));
    assert_eq!(blurred_style.foreground, None);
}

/// Verifies stylesheet variables reuse their stored style expressions.
///
/// # Example Under Test
///
/// ```text
/// $primary: Color::LightCyan;
/// $surface: Color::Black;
/// $pad: TuiSpacing::uniform(1);
/// Text => { fg: $primary, bg: $surface }
/// .panel => { background: $surface, padding: $pad }
/// ```
///
/// # Assertions
///
/// - The macro expands to a stylesheet with a text rule.
/// - The text rule reuses foreground and background variables.
/// - The macro expands to a stylesheet with a panel class rule.
/// - The panel rule reuses background and padding variables.
///
/// # Why
///
/// Stylesheet variables should expand to the same expressions wherever they
/// are referenced.
#[test]
fn stylesheet_macro_variables_reuse_values() {
    let styles = stylesheet! {
        $primary: Color::LightCyan;
        $surface: Color::Black;
        $pad: TuiSpacing::uniform(1);

        Text => { fg: $primary, bg: $surface }
        .panel => { background: $surface, padding: $pad }
    };

    let expected = Stylesheet::new()
        .rule(
            StyleSelector::node_type(NodeType::Text),
            TuiStyle::new()
                .foreground(Color::LightCyan)
                .background(Color::Black),
        )
        .rule(
            StyleSelector::class("panel"),
            TuiStyle::new()
                .background(Color::Black)
                .padding(TuiSpacing::uniform(1)),
        );

    assert_eq!(styles, expected);
}

/// Verifies stylesheet mixins expand into ordinary declarations in source order.
///
/// # Example Under Test
///
/// ```text
/// @mixin control_chrome { fg: Color::White, bg: Color::Blue }
/// Button => { @include control_chrome, fg: Color::Yellow }
/// .primary => { @include control_chrome }
/// ```
///
/// # Assertions
///
/// - The mixin can be reused across two rules.
/// - The mixin expands to ordinary `TuiStyle` builder calls.
/// - Rule-local declarations after the include override mixin defaults.
///
/// # Why
///
/// Mixin reuse should remain a compile-time macro convenience and preserve
/// existing style resolution behavior.
#[test]
fn stylesheet_macro_mixins_expand_in_source_order() {
    let styles = stylesheet! {
        @mixin control_chrome {
            fg: Color::White,
            bg: Color::Blue
        }

        Button => { @include control_chrome, fg: Color::Yellow }
        .primary => { @include control_chrome }
    };

    let expected = Stylesheet::new()
        .rule(
            StyleSelector::node_type(NodeType::Button),
            TuiStyle::new()
                .foreground(Color::White)
                .background(Color::Blue)
                .foreground(Color::Yellow),
        )
        .rule(
            StyleSelector::class("primary"),
            TuiStyle::new()
                .foreground(Color::White)
                .background(Color::Blue),
        );

    assert_eq!(styles, expected);
}
