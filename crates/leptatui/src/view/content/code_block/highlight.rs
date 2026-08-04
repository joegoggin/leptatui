//! Terminal-native syntax-highlighting support for code-block views.

use std::sync::OnceLock;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color as SyntectColor, FontStyle, ThemeSet},
    parsing::SyntaxSet,
};

use crate::style::TERMINAL_SURFACE_BACKGROUND;

/// Bundled Syntect theme used only to classify syntax scopes into palette roles.
const CLASSIFICATION_THEME: &str = "base16-ocean.dark";

/// Base16 Ocean colors paired with terminal-native semantic equivalents.
const TERMINAL_SYNTAX_PALETTE: [((u8, u8, u8), Color); 16] = [
    ((43, 48, 59), Color::DarkGray),
    ((52, 61, 70), Color::DarkGray),
    ((79, 91, 102), Color::Gray),
    ((101, 115, 126), Color::Gray),
    ((167, 173, 186), Color::Gray),
    ((192, 197, 206), Color::White),
    ((223, 225, 232), Color::White),
    ((239, 241, 245), Color::White),
    ((191, 97, 106), Color::LightRed),
    ((208, 135, 112), Color::Yellow),
    ((235, 203, 139), Color::LightYellow),
    ((163, 190, 140), Color::LightGreen),
    ((150, 181, 180), Color::LightCyan),
    ((143, 161, 179), Color::LightBlue),
    ((180, 142, 173), Color::LightMagenta),
    ((171, 121, 103), Color::Red),
];

/// Returns retained logical lines for the requested highlighting configuration.
///
/// Unknown languages and highlighting failures return plain source lines.
///
/// # Arguments
///
/// * `source` — Source text to split and optionally highlight.
/// * `language` — Optional bundled grammar token or alias.
///
/// # Returns
///
/// A [`Vec`] containing at least one owned Ratatui [`Line`].
pub(crate) fn highlighted_source_lines(source: &str, language: Option<&str>) -> Vec<Line<'static>> {
    language
        .and_then(|language| try_highlighted_lines(source, language))
        .unwrap_or_else(|| plain_lines(source))
}

/// Returns the process-wide bundled syntax set.
///
/// # Returns
///
/// A shared [`SyntaxSet`] configured for source lines without newline endings.
fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_nonewlines)
}

/// Returns the process-wide bundled theme set.
///
/// # Returns
///
/// A shared [`ThemeSet`] containing syntect's default themes.
fn theme_set() -> &'static ThemeSet {
    static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Highlights all logical source lines for a recognized language.
///
/// # Arguments
///
/// * `source` — Source text to highlight.
/// * `language` — Grammar token or alias used for lookup.
///
/// # Returns
///
/// An [`Option`] containing highlighted Ratatui lines when lookup and
/// highlighting succeed.
fn try_highlighted_lines(source: &str, language: &str) -> Option<Vec<Line<'static>>> {
    let syntax_set = syntax_set();
    let syntax = syntax_set.find_syntax_by_token(language)?;
    let theme = theme_set().themes.get(CLASSIFICATION_THEME)?;
    let mut highlighter = HighlightLines::new(syntax, theme);
    logical_lines(source)
        .map(|line| {
            highlighter
                .highlight_line(line, syntax_set)
                .ok()
                .map(|ranges| {
                    Line::from(
                        ranges
                            .into_iter()
                            .map(|(style, content)| {
                                Span::styled(content.to_owned(), ratatui_style(style))
                            })
                            .collect::<Vec<_>>(),
                    )
                })
        })
        .collect()
}

/// Converts a syntect style into its Ratatui equivalent.
///
/// # Arguments
///
/// * `style` — Syntect RGB colors and font flags.
///
/// # Returns
///
/// A Ratatui [`Style`] preserving semantic color roles and modifiers.
fn ratatui_style(style: syntect::highlighting::Style) -> Style {
    let mut modifiers = Modifier::empty();
    if style.font_style.contains(FontStyle::BOLD) {
        modifiers |= Modifier::BOLD;
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        modifiers |= Modifier::ITALIC;
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        modifiers |= Modifier::UNDERLINED;
    }

    Style::new()
        .fg(terminal_syntax_color(style.foreground))
        .bg(TERMINAL_SURFACE_BACKGROUND)
        .add_modifier(modifiers)
}

/// Maps a Syntect classification color to the nearest terminal-native role.
///
/// # Arguments
///
/// * `color` — Syntect color whose alpha channel is ignored by terminal output.
///
/// # Returns
///
/// A named Ratatui [`Color`] resolved through the terminal's ANSI palette.
fn terminal_syntax_color(color: SyntectColor) -> Color {
    TERMINAL_SYNTAX_PALETTE
        .iter()
        .min_by_key(|((red, green, blue), _)| {
            let red = i32::from(color.r) - i32::from(*red);
            let green = i32::from(color.g) - i32::from(*green);
            let blue = i32::from(color.b) - i32::from(*blue);
            red * red + green * green + blue * blue
        })
        .map_or(Color::White, |(_, terminal)| *terminal)
}

/// Returns plain owned Ratatui lines for source text.
///
/// # Arguments
///
/// * `source` — Source text to split while preserving trailing blank lines.
///
/// # Returns
///
/// A [`Vec`] containing at least one logical [`Line`].
fn plain_lines(source: &str) -> Vec<Line<'static>> {
    logical_lines(source)
        .map(|line| Line::raw(line.to_owned()))
        .collect()
}

/// Iterates logical source lines while preserving empty and trailing lines.
///
/// # Arguments
///
/// * `source` — Source text to split.
///
/// # Returns
///
/// An iterator of borrowed logical lines.
fn logical_lines(source: &str) -> impl Iterator<Item = &str> {
    source.split('\n')
}
