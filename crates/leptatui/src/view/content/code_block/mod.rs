//! Bordered syntax-highlighted code-block view.
//!
//! # Modules
//!
//! - [`highlight`] — Bundled syntax-set and theme-based source highlighting.

mod highlight;

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use self::highlight::highlighted_source_lines;
use crate::view::core::{
    capabilities::impl_styled_view,
    measurement::{AvailableSpace, cells_to_u16, resolve_intrinsic_axis, sanitize_cells},
    render::{
        horizontal_border_columns, horizontal_padding_columns, line_count_height, resolve_style,
        vertical_border_rows, vertical_padding_rows,
    },
};
use crate::view::{StyleMetadata, View, ViewType};
use crate::{Borders, LayoutSize, TuiStyle, app::Result, component::RenderCtx};

/// Bundled syntax and background theme used by code-block views.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SyntaxTheme {
    /// Base16 Ocean dark theme.
    #[default]
    Dark,
    /// Base16 Ocean light theme.
    Light,
}

/// Bordered syntax-highlighted source code.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeBlockView {
    /// Original source used when highlighting configuration changes.
    pub(crate) source: String,
    /// Caller-supplied language token.
    pub(crate) language: Option<String>,
    /// Whether one-based line numbers are displayed.
    pub(crate) line_numbers: bool,
    /// Bundled syntax theme used for recognized source.
    pub(crate) syntax_theme: SyntaxTheme,
    /// Retained highlighted logical source lines.
    pub(crate) highlighted_lines: Vec<ratatui::text::Line<'static>>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

impl CodeBlockView {
    /// Sets the language token used for syntax highlighting.
    ///
    /// # Arguments
    ///
    /// * `language` — Grammar token or alias to select and display.
    ///
    /// # Returns
    ///
    /// This code block with refreshed highlighted lines.
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self.highlighted_lines =
            highlighted_source_lines(&self.source, self.language.as_deref(), self.syntax_theme);
        self
    }

    /// Sets whether one-based line numbers are displayed.
    ///
    /// # Arguments
    ///
    /// * `line_numbers` — Whether to render a one-based line-number gutter.
    ///
    /// # Returns
    ///
    /// This code block with the requested line-number setting.
    pub fn line_numbers(mut self, line_numbers: bool) -> Self {
        self.line_numbers = line_numbers;
        self
    }

    /// Sets the bundled syntax-highlighting theme.
    ///
    /// # Arguments
    ///
    /// * `syntax_theme` — Bundled theme used to highlight recognized source.
    ///
    /// # Returns
    ///
    /// This code block with refreshed highlighted lines.
    pub fn syntax_theme(mut self, syntax_theme: SyntaxTheme) -> Self {
        self.syntax_theme = syntax_theme;
        self.highlighted_lines =
            highlighted_source_lines(&self.source, self.language.as_deref(), self.syntax_theme);
        self
    }
}

/// Creates a bordered syntax-highlighted code block.
///
/// # Arguments
///
/// * `source` — Source text retained for highlighting and rendering.
///
/// # Returns
///
/// A [`CodeBlockView`] using the dark theme with line numbers disabled.
pub fn code_block(source: impl Into<String>) -> CodeBlockView {
    let source = source.into();
    CodeBlockView {
        highlighted_lines: highlighted_source_lines(&source, None, SyntaxTheme::Dark),
        source,
        language: None,
        line_numbers: false,
        syntax_theme: SyntaxTheme::Dark,
        metadata: StyleMetadata::new(ViewType::CodeBlock),
    }
}

impl CodeBlockView {
    /// Returns the original source text.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the optional language token.
    pub fn language_token(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Returns whether line numbers are enabled.
    pub const fn has_line_numbers(&self) -> bool {
        self.line_numbers
    }

    /// Returns the selected syntax theme.
    pub const fn selected_syntax_theme(&self) -> SyntaxTheme {
        self.syntax_theme
    }

    /// Returns retained highlighted logical source lines.
    pub fn highlighted_lines(&self) -> &[ratatui::text::Line<'static>] {
        &self.highlighted_lines
    }
}

/// Syntax spans are split only at grapheme boundaries. When line numbers are
/// enabled, each logical line begins with a right-aligned `number │ ` gutter
/// and continuation rows receive an equally wide blank gutter.
///
/// # Arguments
///
/// * `lines` — Retained highlighted logical source lines.
/// * `line_numbers` — Whether the logical-line gutter is enabled.
/// * `width` — Available terminal-cell width inside the code-block border.
/// * `style` — Resolved code-block style inherited by unstyled content.
///
/// # Returns
///
/// A Ratatui [`Text`] containing width-aware visual rows.
fn wrapped_code_text(
    lines: &[Line<'static>],
    line_numbers: bool,
    width: u16,
    style: TuiStyle,
) -> Text<'static> {
    let digits = lines.len().max(1).to_string().len();
    let gutter_width = if line_numbers {
        u16::try_from(digits.saturating_add(3)).unwrap_or(u16::MAX)
    } else {
        0
    };
    let code_width = width.saturating_sub(gutter_width);
    let base_style = style.to_ratatui_style();
    let mut visual_lines = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let wrapped = wrap_styled_line(line, code_width, base_style);
        for (visual_index, wrapped_line) in wrapped.into_iter().enumerate() {
            if line_numbers {
                let gutter = if visual_index == 0 {
                    format!("{:>digits$} │ ", index.saturating_add(1))
                } else {
                    " ".repeat(digits.saturating_add(3))
                };
                let mut spans = vec![Span::styled(gutter, base_style)];
                spans.extend(wrapped_line.spans);
                visual_lines.push(Line::from(spans));
            } else {
                visual_lines.push(wrapped_line);
            }
        }
    }

    Text::from(visual_lines)
}

/// Returns wrapped code content and its required block height.
///
/// # Arguments
///
/// * `highlighted_lines` — Retained highlighted logical source lines.
/// * `line_numbers` — Whether the logical-line gutter is enabled.
/// * `style` — Resolved code-block style used for wrapping and block geometry.
/// * `area` — Available code-block render area.
///
/// # Returns
///
/// A tuple containing wrapped visual lines and the saturated required height.
fn code_block_layout(
    highlighted_lines: &[Line<'static>],
    line_numbers: bool,
    style: TuiStyle,
    area: Rect,
) -> (Text<'static>, u16) {
    let inner = style
        .to_block_with_default_borders(Borders::ALL)
        .inner(area);
    let content = wrapped_code_text(highlighted_lines, line_numbers, inner.width, style);
    let required_height = line_count_height(content.lines.len())
        .max(1)
        .saturating_add(vertical_border_rows(style.borders.unwrap_or(Borders::ALL)))
        .saturating_add(vertical_padding_rows(style.padding));

    (content, required_height)
}

/// Wraps one styled logical line at grapheme boundaries.
///
/// # Arguments
///
/// * `line` — Styled logical line to wrap.
/// * `width` — Available content width.
/// * `base_style` — Resolved style inherited beneath syntax spans.
///
/// # Returns
///
/// A non-empty [`Vec`] of visual [`Line`] values.
pub(crate) fn wrap_styled_line(
    line: &Line<'static>,
    width: u16,
    base_style: Style,
) -> Vec<Line<'static>> {
    let width = usize::from(width);
    if width == 0 {
        return vec![Line::default()];
    }

    let mut wrapped = Vec::new();
    let mut spans = Vec::new();
    let mut line_width = 0usize;
    for grapheme in line.styled_graphemes(base_style) {
        let grapheme_width = UnicodeWidthStr::width(grapheme.symbol);
        if grapheme_width > width {
            if !spans.is_empty() {
                wrapped.push(Line::from(std::mem::take(&mut spans)));
                line_width = 0;
            }
            continue;
        }
        if line_width.saturating_add(grapheme_width) > width && !spans.is_empty() {
            wrapped.push(Line::from(std::mem::take(&mut spans)));
            line_width = 0;
        }
        let grapheme_style = base_style
            .bg
            .map_or(grapheme.style, |background| grapheme.style.bg(background));
        spans.push(Span::styled(grapheme.symbol.to_owned(), grapheme_style));
        line_width = line_width.saturating_add(grapheme_width);
    }
    wrapped.push(Line::from(spans));
    wrapped
}

/// Renders a bordered syntax-highlighted code block with a uniform interior background.
///
/// # Arguments
///
/// * `language` — Optional caller-supplied token shown in the border title.
/// * `line_numbers` — Whether the logical-line gutter is enabled.
/// * `syntax_theme` — Bundled theme supplying the default block background.
/// * `highlighted_lines` — Retained highlighted logical source lines.
/// * `metadata` — Selector metadata used to resolve block styling.
/// * `ctx` — Rendering context containing the target area.
///
/// # Returns
///
/// An empty [`Result`] on success.
fn render_code_block_view(
    language: Option<&str>,
    line_numbers: bool,
    syntax_theme: SyntaxTheme,
    highlighted_lines: &[Line<'static>],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let style = resolve_style(metadata, ctx);
    let background = style
        .background
        .unwrap_or_else(|| syntax_theme.background());
    let mut content_style = style;
    content_style.background = Some(background);
    let area = ctx.area();
    let (content, required_height) =
        code_block_layout(highlighted_lines, line_numbers, content_style, area);
    let mut visible_style = style;
    if area.height < required_height {
        let mut borders = style.borders.unwrap_or(Borders::ALL);
        borders.remove(Borders::BOTTOM);
        visible_style.borders = Some(borders);
    }
    visible_style.background = None;
    let mut background_area_style = visible_style;
    background_area_style.padding = None;
    let background_area = background_area_style
        .to_block_with_default_borders(Borders::ALL)
        .inner(area);
    let block = visible_style.to_block_with_default_borders(Borders::ALL);
    let block = if let Some(language) = language {
        block.title(language.to_owned())
    } else {
        block
    };
    let inner = block.inner(area);
    ctx.with_area(background_area, |ctx| {
        ctx.render_widget(Block::new().style(Style::new().bg(background)));
    });
    ctx.render_widget(block);
    ctx.with_area_inherited_style_and_selector_ancestor(
        inner,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            ctx.render_widget(Paragraph::new(content).style(content_style.to_ratatui_style()));
        },
    );
    Ok(())
}

impl View for CodeBlockView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        render_code_block_view(
            self.language.as_deref(),
            self.line_numbers,
            self.syntax_theme,
            &self.highlighted_lines,
            &self.metadata,
            ctx,
        )
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let style = resolve_style(&self.metadata, ctx);
        let borders = style.borders.unwrap_or(Borders::ALL);
        let horizontal_inset = horizontal_border_columns(borders)
            .saturating_add(horizontal_padding_columns(style.padding));
        let gutter_width = if self.line_numbers {
            u16::try_from(
                self.highlighted_lines
                    .len()
                    .max(1)
                    .to_string()
                    .len()
                    .saturating_add(3),
            )
            .unwrap_or(u16::MAX)
        } else {
            0
        };
        let max_code_width = self
            .highlighted_lines
            .iter()
            .map(|line| u16::try_from(line.width()).unwrap_or(u16::MAX))
            .max()
            .unwrap_or(0);
        let min_code_width = u16::from(!self.highlighted_lines.is_empty());
        let width = resolve_intrinsic_axis(
            known_dimensions.width,
            available_space.width,
            f32::from(
                min_code_width
                    .saturating_add(gutter_width)
                    .saturating_add(horizontal_inset),
            ),
            f32::from(
                max_code_width
                    .saturating_add(gutter_width)
                    .saturating_add(horizontal_inset),
            ),
        );
        let natural_height = code_block_layout(
            &self.highlighted_lines,
            self.line_numbers,
            style,
            Rect::new(0, 0, cells_to_u16(width), u16::MAX),
        )
        .1;
        let height = known_dimensions
            .height
            .map_or(f32::from(natural_height), sanitize_cells);
        LayoutSize::new(width, height)
    }

    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl_styled_view!(CodeBlockView);
