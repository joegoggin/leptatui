//! Markdown fixture integration tests.
//!
//! These tests exercise the public Markdown reader APIs against representative
//! documents and verify both semantic view construction and stable fragments of
//! terminal-buffer output.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use leptatui::{
    AnyView, AvailableSpace, Borders, CellAlignment, CodeBlockView, Color, DivView, IntoView,
    IntoViews, LayoutSize, MarkdownOptions, Modifier, RenderCtx, TuiSpacing, TuiStyle, View, block,
    code_block, div, h1, h2, h3, h4, h5, h6, list_item, markdown, markdown_with_options,
    ordered_list, paragraph, table, table_body, table_cell, table_head, table_row, unordered_list,
    view::{Line, Span, Text},
};
use ratatui::{Terminal, backend::TestBackend, style::Style};

use crate::support::{render_view, rendered_lines};

/// Representative headings, paragraphs, inline syntax, and list fixture.
const CORE_FIXTURE: &str = include_str!("../fixtures/markdown/core.md");
/// Representative readable fallback and table fixture.
const FALLBACKS_FIXTURE: &str = include_str!("../fixtures/markdown/fallbacks.md");
/// Representative fenced, indented, highlighted, and fallback code fixture.
const CODE_FIXTURE: &str = include_str!("../fixtures/markdown/code.md");
/// Zero-content Markdown fixture.
const EMPTY_FIXTURE: &str = include_str!("../fixtures/markdown/empty.md");

/// Creates the expected semantic sequence for visibly separated Markdown blocks.
///
/// # Arguments
///
/// * `blocks` — Content-bearing Markdown blocks in source order.
///
/// # Returns
///
/// A [`Vec`] containing one empty paragraph between each block.
fn separated_blocks(blocks: impl IntoViews) -> Vec<AnyView> {
    let blocks = blocks.into_views();
    let mut separated = Vec::with_capacity(blocks.len().saturating_mul(2).saturating_sub(1));

    for block in blocks {
        if !separated.is_empty() {
            separated.push(paragraph("").into_view());
        }
        separated.push(block);
    }

    separated
}

/// Creates the semantic blockquote fallback used by Markdown conversion.
///
/// # Arguments
///
/// * `children` — Semantic children nested inside the quote.
///
/// # Returns
///
/// A left-bordered [`View`] matching the public Markdown presentation.
fn block_quote(children: impl IntoViews) -> impl View {
    block(div(children)).with_inline_style(
        TuiStyle::new()
            .borders(Borders::LEFT)
            .padding(TuiSpacing::new(1, 0, 0, 0)),
    )
}

/// Creates the semantic thematic-break fallback used by Markdown conversion.
///
/// # Returns
///
/// A top-bordered [`View`] matching the public Markdown presentation.
fn thematic_break() -> impl View {
    block(div(())).with_inline_style(TuiStyle::new().borders(Borders::TOP))
}

/// Asserts two view trees produce the same terminal output.
fn assert_views_render_equally(actual: &AnyView, expected: &dyn View) {
    let actual = render_view(actual.as_view(), 80, 200).expect("actual view should render");
    let expected = render_view(expected, 80, 200).expect("expected view should render");

    assert_eq!(rendered_lines(&actual), rendered_lines(&expected));
}

include!("code.rs");
include!("core.rs");
include!("fallback.rs");
include!("rendering.rs");
