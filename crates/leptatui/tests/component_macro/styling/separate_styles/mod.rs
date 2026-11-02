//! Cross-file component stylesheet fixture.
//!
//! # Modules
//!
//! - [`style`] — Fixture stylesheet registration.

mod style;

use leptatui::{component, div, text};

use self::style::use_separate_style_fixture_styles;

/// Renders a component whose stylesheet is registered from a sibling module.
///
/// # Returns
///
/// A styled root and BEM content element.
#[component]
fn MacroSeparateStyledText() -> impl IntoView {
    use_separate_style_fixture_styles();

    div((
        text("Styled").with_classes("separate-style__content"),
    ))
        .with_classes("separate-style")
}

/// Renders styled and plain siblings that share the fixture's BEM class.
///
/// # Returns
///
/// A root containing one scoped styled component and one plain text view.
#[component]
pub(super) fn MacroSeparateStyleRoot() -> impl IntoView {
    div((
        component(MacroSeparateStyledText::new()),
        text("Plain").with_classes("separate-style__content"),
    ))
}
