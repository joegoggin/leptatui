//! Pass fixture for imported `stylesheet!` variables and mixins.

use leptatui::prelude::*;

/// Defines reusable color variables.
fn color_variables() -> StyleModule {
    stylesheet! {
        $fg: Color::Black;
        $bg: Color::White;
        $border: Borders::ALL;
        $border_type: BorderType::Rounded;
        $padding: TuiSpacing::uniform(1);
    }
}

/// Defines reusable button mixins from imported variables.
fn button_mixins() -> StyleModule {
    stylesheet! {
        @use color_variables;

        @mixin primary {
            fg: color_variables.$fg,
            bg: color_variables.$bg,
            borders: color_variables.$border,
            border_type: color_variables.$border_type,
            padding: color_variables.$padding
        }

        @mixin inverted {
            @include primary,
            fg: color_variables.$bg,
            bg: color_variables.$fg
        }
    }
}

/// Exercises imported mixins in a rule-producing stylesheet.
fn main() {
    let styles = stylesheet! {
        @use button_mixins;

        .submit => { @include button_mixins.primary }
        .quit => { @include button_mixins.inverted }
    };

    let base = TuiStyle::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(TuiSpacing::uniform(1));
    let expected = Stylesheet::new()
        .rule(
            StyleSelector::class("submit"),
            base.foreground(Color::Black).background(Color::White),
        )
        .rule(
            StyleSelector::class("quit"),
            base.foreground(Color::Black)
                .background(Color::White)
                .foreground(Color::White)
                .background(Color::Black),
        );

    assert_eq!(styles, expected);
}
