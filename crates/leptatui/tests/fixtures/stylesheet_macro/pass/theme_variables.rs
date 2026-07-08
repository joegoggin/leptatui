//! Pass fixture for runtime theme variables in `stylesheet!`.

use leptatui::prelude::*;

/// Exercises theme variable references in stylesheet declarations.
fn main() {
    let styles = stylesheet! {
        $text: theme_color("text");
        $surface: theme_color("surface");

        .panel => { fg: $text, bg: $surface }
    };

    let expected = Stylesheet::new().rule(
        StyleSelector::class("panel"),
        StyleDeclarations::new()
            .foreground(theme_color("text"))
            .background(theme_color("surface")),
    );

    assert_eq!(styles, expected);
}
