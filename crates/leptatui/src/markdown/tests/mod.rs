//! Tests for Markdown parsing and semantic view conversion.

use std::{
    fs, io,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Modifier, Style},
    text::{Line, Span, Text},
};

use super::block::{block_quote, separate_blocks, thematic_break};
use super::*;
use crate::*;

/// Erases heterogeneous test views into one child vector.
macro_rules! views {
        ($($view:expr),* $(,)?) => {
            vec![$($view.into_view()),*]
        };
    }

/// Returns a unique temporary directory path for Markdown reader fixtures.
///
/// # Arguments
///
/// * `name` — Human-readable suffix identifying the fixture purpose.
///
/// # Returns
///
/// A [`PathBuf`] below the process temporary directory.
fn markdown_fixture_dir(name: &str) -> PathBuf {
    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    std::env::temp_dir().join(format!(
        "leptatui-markdown-{}-{}-{name}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Returns code-block options from a single-block Markdown document.
///
/// # Arguments
///
/// * `view` — Parsed document expected to contain one code block.
///
/// # Returns
///
/// A tuple containing line-number visibility and the syntax theme.
fn parsed_code_block_options(view: &AnyView) -> (bool, SyntaxTheme) {
    let document = view
        .downcast_ref::<LayoutView>()
        .expect("Markdown document should be a column layout");
    let [child] = document.children() else {
        panic!("expected one Markdown code block");
    };
    let code = child
        .downcast_ref::<CodeBlockView>()
        .expect("Markdown child should be a code block");

    (code.has_line_numbers(), code.selected_syntax_theme())
}

/// Returns scroll offset and maximum offset from a Markdown document.
///
/// # Arguments
///
/// * `view` — Parsed Markdown column whose scroll metadata is inspected.
///
/// # Returns
///
/// A tuple containing the current and maximum vertical scroll offsets.
fn markdown_scroll_state(view: &AnyView) -> (u16, u16) {
    let document = view
        .downcast_ref::<LayoutView>()
        .expect("Markdown document should be a column layout");
    let metadata = document.metadata();

    (metadata.scroll_offset(), metadata.max_scroll_offset())
}

/// Renders a view into fixed terminal rows for fallback assertions.
///
/// # Arguments
///
/// * `view` — View tree to render.
/// * `width` — Test terminal width in cells.
/// * `height` — Test terminal height in cells.
///
/// # Returns
///
/// A [`Vec`] containing rendered terminal symbols grouped by row.
///
/// # Errors
///
/// Returns [`crate::Error`] if terminal or view rendering fails.
fn rendered_view_lines(view: &AnyView, width: u16, height: u16) -> Result<Vec<String>> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    let cells = terminal.backend().buffer().content();
    Ok(cells
        .chunks(usize::from(width))
        .map(|row| row.iter().map(|cell| cell.symbol()).collect())
        .collect())
}

/// Renders Markdown into fixed terminal rows for fallback assertions.
///
/// # Arguments
///
/// * `source` — CommonMark source to convert and render.
/// * `width` — Test terminal width in cells.
/// * `height` — Test terminal height in cells.
///
/// # Returns
///
/// A [`Vec`] containing rendered terminal symbols grouped by row.
///
/// # Errors
///
/// Returns [`crate::Error`] if terminal or view rendering fails.
fn rendered_markdown_lines(source: &str, width: u16, height: u16) -> Result<Vec<String>> {
    rendered_view_lines(&markdown(source), width, height)
}

include!("api.rs");
include!("rendering.rs");
include!("code.rs");
include!("inline.rs");
include!("structures.rs");
include!("fallbacks.rs");
include!("ordering.rs");
include!("navigation.rs");
