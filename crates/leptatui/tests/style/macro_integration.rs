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
/// terminal-view focus matching.
#[test]
fn stylesheet_macro_nested_selectors_resolve_against_ancestors() {
    let styles = stylesheet! {
        .panel => {
            Button => {
                &:focus => { fg: Color::Yellow }
            }
        }
    };
    let mut panel = StyleMetadata::new(ViewType::Block);
    panel.set_classes("panel");
    let focused = button("Save").with_focus(true);
    let blurred = button("Cancel");

    let focused_style = styles.resolve(
        focused.style_metadata().unwrap(),
        &[panel.clone()],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let blurred_style = styles.resolve(
        blurred.style_metadata().unwrap(),
        &[panel],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

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
            StyleSelector::view_type(ViewType::Text),
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
            StyleSelector::view_type(ViewType::Button),
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
