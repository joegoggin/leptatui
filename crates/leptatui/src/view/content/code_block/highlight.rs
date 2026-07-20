//! Syntax-highlighting support for code-block views.

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

use super::SyntaxTheme;

impl SyntaxTheme {
    /// Returns the syntect bundled-theme key for this public theme.
    ///
    /// # Returns
    ///
    /// A string slice key present in syntect's default theme set.
    fn key(self) -> &'static str {
        match self {
            Self::Dark => "base16-ocean.dark",
            Self::Light => "base16-ocean.light",
        }
    }

    /// Returns the bundled theme's terminal background color.
    ///
    /// Syntect's white fallback is retained if a bundled theme unexpectedly
    /// omits its background setting.
    ///
    /// # Returns
    ///
    /// A Ratatui [`Color`] for the selected syntax theme.
    pub(crate) fn background(self) -> Color {
        let background = theme_set()
            .themes
            .get(self.key())
            .and_then(|theme| theme.settings.background)
            .unwrap_or(SyntectColor::WHITE);
        ratatui_color(background)
    }
}

/// Returns retained logical lines for the requested highlighting configuration.
///
/// Unknown languages and highlighting failures return plain source lines.
///
/// # Arguments
///
/// * `source` — Source text to split and optionally highlight.
/// * `language` — Optional bundled grammar token or alias.
/// * `syntax_theme` — Bundled theme used for recognized source.
///
/// # Returns
///
/// A [`Vec`] containing at least one owned Ratatui [`Line`].
pub(crate) fn highlighted_source_lines(
    source: &str,
    language: Option<&str>,
    syntax_theme: SyntaxTheme,
) -> Vec<Line<'static>> {
    language
        .and_then(|language| try_highlighted_lines(source, language, syntax_theme))
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
/// * `syntax_theme` — Bundled theme to apply.
///
/// # Returns
///
/// An [`Option`] containing highlighted Ratatui lines when lookup and
/// highlighting succeed.
fn try_highlighted_lines(
    source: &str,
    language: &str,
    syntax_theme: SyntaxTheme,
) -> Option<Vec<Line<'static>>> {
    let syntax_set = syntax_set();
    let syntax = syntax_set.find_syntax_by_token(language)?;
    let theme = theme_set().themes.get(syntax_theme.key())?;
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
/// A Ratatui [`Style`] preserving foreground, background, and modifiers.
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
        .fg(ratatui_color(style.foreground))
        .bg(ratatui_color(style.background))
        .add_modifier(modifiers)
}

/// Converts a Syntect RGB color into its Ratatui equivalent.
///
/// # Arguments
///
/// * `color` — Syntect color whose alpha channel is ignored by terminal output.
///
/// # Returns
///
/// A Ratatui [`Color`] retaining the red, green, and blue components.
fn ratatui_color(color: SyntectColor) -> Color {
    Color::Rgb(color.r, color.g, color.b)
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
