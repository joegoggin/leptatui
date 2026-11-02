/// Verifies nested stylesheet macro rules resolve against ancestor metadata.
///
/// # Example Under Test
///
/// ```text
/// .panel => {
///     Button => {
///         &:focus => { fg: Color::Yellow }
///     }
///     A => {
///         &:active => { fg: Color::LightCyan }
///     }
///     Input => {
///         &:insert => { fg: Color::Yellow }
///     }
///     TextArea => {
///         &:visual => { fg: Color::Magenta }
///     }
///     Link => {
///         &:visited => { fg: Color::LightMagenta }
///     }
/// }
/// ```
///
/// # Assertions
///
/// - The macro accepts nested rules with `&:focus`, `&:active`, `&:insert`, and
///   `&:visual`, and `&:visited`.
/// - The focused button resolves to a yellow foreground under `.panel`.
/// - The blurred button retains its default white foreground under `.panel`.
/// - The active anchor resolves to a light-cyan foreground under `.panel`.
/// - The insert-mode input resolves to a yellow foreground under `.panel`.
/// - The visual-mode text area resolves to a magenta foreground under `.panel`.
/// - The visited link resolves to a light-magenta foreground under `.panel`.
///
/// # Why
///
/// Nested macro selectors should lower into descendant selectors that preserve
/// terminal-view pseudo-class matching.
#[test]
fn stylesheet_macro_nested_selectors_resolve_against_ancestors() {
    let styles = stylesheet! {
        .panel => {
            Button => {
                &:focus => { fg: Color::Yellow }
            }
            A => {
                &:active => { fg: Color::LightCyan }
            }
            Input => {
                &:insert => { fg: Color::Yellow }
            }
            TextArea => {
                &:visual => { fg: Color::Magenta }
            }
            Link => {
                &:visited => { fg: Color::LightMagenta }
            }
        }
    };
    let mut panel = StyleMetadata::new(ViewType::Block);
    panel.set_classes("panel");
    let focused = button("Save").with_focus(true);
    let blurred = button("Cancel");
    let mut active_anchor = StyleMetadata::new(ViewType::A);
    active_anchor.set_active(true);
    let mut insert_input = StyleMetadata::new(ViewType::Input);
    insert_input.set_insert(true);
    let mut visual_text_area = StyleMetadata::new(ViewType::TextArea);
    visual_text_area.set_visual(true);
    let mut visited_link = StyleMetadata::new(ViewType::Link);
    visited_link.set_visited(true);

    let focused_style = styles.resolve(
        focused.style_metadata().unwrap(),
        &[panel.clone()],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let blurred_style = styles.resolve(
        blurred.style_metadata().unwrap(),
        &[panel.clone()],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let active_style = styles.resolve(
        &active_anchor,
        &[panel.clone()],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let insert_style = styles.resolve(
        &insert_input,
        &[panel.clone()],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let visual_style = styles.resolve(
        &visual_text_area,
        &[panel.clone()],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let visited_style = styles.resolve(
        &visited_link,
        &[panel],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(focused_style.foreground, Some(Color::Yellow));
    assert_eq!(blurred_style.foreground, Some(Color::White));
    assert_eq!(active_style.foreground, Some(Color::LightCyan));
    assert_eq!(insert_style.foreground, Some(Color::Yellow));
    assert_eq!(visual_style.foreground, Some(Color::Magenta));
    assert_eq!(visited_style.foreground, Some(Color::LightMagenta));
}

/// Verifies nested BEM suffixes concatenate with their parent class selector.
///
/// # Example Under Test
///
/// ```text
/// .example-page => {
///     &__button => {
///         &:focus => { bg: Color::Black }
///         &--submit => { fg: Color::Green }
///         &--cancel => { fg: Color::Red }
///     }
/// }
/// ```
///
/// # Assertions
///
/// - Element and modifier suffixes concatenate into BEM class names.
/// - The focused base button receives its pseudo-class background.
/// - Submit and cancel modifiers receive their distinct foreground colors.
/// - The generated selectors do not require an `.example-page` ancestor.
///
/// # Why
///
/// Component-owned styles should support readable BEM grouping without
/// changing the runtime selector model or adding artificial ancestry.
#[test]
fn stylesheet_macro_bem_parent_suffixes_concatenate_class_names() {
    let styles = stylesheet! {
        .example-page => {
            &__button => {
                &:focus => { bg: Color::Black }
                &--submit => { fg: Color::Green }
                &--cancel => { fg: Color::Red }
            }
        }
    };
    let focused = button("Focused").with_classes("example-page__button").with_focus(true);
    let submit = button("Submit").with_classes("example-page__button example-page__button--submit");
    let cancel = button("Cancel").with_classes("example-page__button example-page__button--cancel");
    let theme = ThemeVariables::new();

    let focused_style = styles.resolve(
        focused.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &theme,
    );
    let submit_style = styles.resolve(
        submit.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &theme,
    );
    let cancel_style = styles.resolve(
        cancel.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &theme,
    );

    assert_eq!(focused_style.background, Some(Color::Black));
    assert_eq!(submit_style.foreground, Some(Color::Green));
    assert_eq!(cancel_style.foreground, Some(Color::Red));
}

/// Verifies nested media blocks inherit their containing selector path.
///
/// # Example Under Test
///
/// ```text
/// .panel => {
///     fg: White
///     @media (max-width: 60) {
///         fg: Yellow
///         Button => { padding: Zero }
///     }
///     &__actions => {
///         flex_direction: Row
///         @media (max-width: 60) { flex_direction: Column }
///     }
/// }
/// ```
///
/// # Assertions
///
/// - The panel uses its responsive foreground only at the matching width.
/// - The descendant button receives responsive padding under the panel.
/// - The BEM action class resolves its nested responsive direction.
///
/// # Why
///
/// Nested media syntax must retain the same selector paths and viewport
/// behavior as equivalent top-level media rules.
#[test]
fn stylesheet_macro_nested_media_inherits_selector_paths() {
    let styles = stylesheet! {
        .panel => {
            fg: Color::White

            @media (max-width: 60) {
                fg: Color::Yellow
                Button => { padding: TuiSpacing::ZERO }
            }

            &__actions => {
                flex_direction: FlexDirection::Row

                @media (max-width: 60) {
                    flex_direction: FlexDirection::Column
                }
            }
        }
    };
    let panel = text("Panel").with_classes("panel");
    let button = button("Save");
    let mut actions = StyleMetadata::new(ViewType::Div);
    actions.set_classes("panel__actions");
    let mut panel_ancestor = StyleMetadata::new(ViewType::Div);
    panel_ancestor.set_classes("panel");
    let theme = ThemeVariables::new();

    let compact_panel = styles.resolve_for_viewport(
        panel.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        ViewportSize::new(60, 20),
        &theme,
    );
    let wide_panel = styles.resolve_for_viewport(
        panel.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        ViewportSize::new(61, 20),
        &theme,
    );
    let compact_button = styles.resolve_for_viewport(
        button.style_metadata().unwrap(),
        &[panel_ancestor],
        TuiStyle::new(),
        ViewportSize::new(60, 20),
        &theme,
    );
    let compact_actions = styles.resolve_for_viewport(
        &actions,
        &[],
        TuiStyle::new(),
        ViewportSize::new(60, 20),
        &theme,
    );

    assert_eq!(compact_panel.foreground, Some(Color::Yellow));
    assert_eq!(wide_panel.foreground, Some(Color::White));
    assert_eq!(compact_button.padding, Some(TuiSpacing::ZERO));
    assert_eq!(
        compact_actions.flex_direction,
        Some(FlexDirection::Column)
    );
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
