//! Tests for Markdown parsing and semantic view conversion.

use std::{
    io,
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

/// Verifies in-memory Markdown readers apply default and custom options.
///
/// # Example Under Test
///
/// ```text
/// markdown("```rust\nfn main() {}\n```")
/// markdown_with_options(source, light theme + line numbers)
/// ```
///
/// # Assertions
///
/// - Both in-memory readers return document views without failure.
/// - Default code blocks use the dark theme without line numbers.
/// - Custom options apply the light theme and enable line numbers.
/// - An owned source string is accepted by the option-bearing reader.
#[test]
fn markdown_reader_apis_apply_default_and_custom_options() {
    let source = "```rust\nfn main() {}\n```\n";
    let default = markdown(source);
    assert_eq!(
        parsed_code_block_options(&default),
        (false, SyntaxTheme::Dark)
    );

    let options = MarkdownOptions::default()
        .syntax_theme(SyntaxTheme::Light)
        .line_numbers(true);
    let owned_source = source.to_owned();
    let configured = markdown_with_options(owned_source, options);
    assert_eq!(
        parsed_code_block_options(&configured),
        (true, SyntaxTheme::Light)
    );
}

/// Verifies Markdown file readers synchronously load UTF-8 source.
///
/// # Example Under Test
///
/// ```text
/// markdown_file("guide.md")
/// markdown_file_with_options("guide.md", light theme + line numbers)
/// ```
///
/// # Assertions
///
/// - The UTF-8 fixture writes and both file readers load it successfully.
/// - The default file reader matches the in-memory default reader.
/// - The option-bearing file reader applies its code-block defaults.
/// - The fixture directory is removed after verification.
#[test]
fn markdown_file_reader_apis_load_utf8_source() {
    let fixture_dir = markdown_fixture_dir("readers");
    let fixture_path = fixture_dir.join("guide.md");
    let source = "```rust\nfn main() {}\n```\n";
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    fs::write(&fixture_path, source).expect("Markdown fixture should be written");

    let default = markdown_file(&fixture_path);
    assert_eq!(default, markdown(source));

    let options = MarkdownOptions::default()
        .syntax_theme(SyntaxTheme::Light)
        .line_numbers(true);
    let configured = markdown_file_with_options(&fixture_path, options);
    assert_eq!(
        parsed_code_block_options(&configured),
        (true, SyntaxTheme::Light)
    );

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
}

/// Verifies Markdown file failures become path-aware semantic fallbacks.
///
/// # Example Under Test
///
/// ```text
/// missing.md
/// directory.md/
/// invalid-utf8.md containing FF FE
/// ```
///
/// # Assertions
///
/// - Missing paths produce a paragraph containing the path and not-found error.
/// - Directory paths produce a paragraph containing their platform I/O failure.
/// - Invalid UTF-8 produces a paragraph containing the path and decoding error.
/// - Every failure remains inside a scrollable document column.
/// - The missing-file fallback renders visibly without propagating an error.
/// - The fixture directory is removed after verification.
#[test]
fn markdown_file_failures_render_path_aware_fallbacks() {
    let fixture_dir = markdown_fixture_dir("errors");
    let directory_path = fixture_dir.join("directory.md");
    let invalid_utf8_path = fixture_dir.join("invalid-utf8.md");
    let missing_path = fixture_dir.join("missing.md");
    fs::create_dir_all(&directory_path).expect("directory fixture should be created");
    fs::write(&invalid_utf8_path, [0xff, 0xfe]).expect("invalid UTF-8 fixture should be written");

    let expected_fallback = |path: &Path, error: &io::Error| {
        column([paragraph(format!(
            "failed to read Markdown file `{}`: {error}",
            path.display()
        ))])
    };

    let missing_error =
        fs::read_to_string(&missing_path).expect_err("missing fixture should fail to read");
    assert_eq!(missing_error.kind(), io::ErrorKind::NotFound);
    let missing = markdown_file(&missing_path);
    assert_eq!(missing, expected_fallback(&missing_path, &missing_error));
    let rendered = rendered_view_lines(&missing, 120, 2)
        .expect("missing-file fallback should render without failure")
        .concat();
    assert!(rendered.contains("failed to read Markdown file"));
    assert!(rendered.contains("missing.md"));

    let directory_error =
        fs::read_to_string(&directory_path).expect_err("directory fixture should fail to read");
    assert_ne!(directory_error.kind(), io::ErrorKind::NotFound);
    assert_eq!(
        markdown_file(&directory_path),
        expected_fallback(&directory_path, &directory_error)
    );

    let invalid_utf8_error = fs::read_to_string(&invalid_utf8_path)
        .expect_err("invalid UTF-8 fixture should fail to read");
    assert_eq!(invalid_utf8_error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(
        markdown_file_with_options(&invalid_utf8_path, MarkdownOptions::default()),
        expected_fallback(&invalid_utf8_path, &invalid_utf8_error)
    );

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
}

/// Verifies in-memory Markdown rendering never interprets source as a path.
///
/// # Example Under Test
///
/// ```text
/// markdown("/temporary/missing.md")
/// ```
///
/// # Assertions
///
/// - The path does not exist before or after conversion and rendering.
/// - The path-like source becomes an ordinary Markdown paragraph.
/// - Rendering succeeds without filesystem access.
#[test]
fn markdown_source_rendering_performs_no_filesystem_io() -> Result<()> {
    let missing_path = markdown_fixture_dir("no-io").join("missing.md");
    let source = missing_path.display().to_string();
    assert!(!missing_path.exists());

    let view = markdown(&source);
    assert_eq!(view, column([paragraph(source)]));
    let mut terminal = Terminal::new(TestBackend::new(80, 2))?;
    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert!(!missing_path.exists());
    Ok(())
}

/// Verifies Markdown documents use the existing vertical scroll commands.
///
/// # Example Under Test
///
/// ```text
/// ten Markdown paragraphs rendered into a 3-row terminal
/// Down, Up, PageDown, PageUp, G, gg
/// ```
///
/// # Assertions
///
/// - Rendering establishes an overflowing scroll range on the document column.
/// - Arrow keys move one row down and up.
/// - Page keys move five rows down and up.
/// - `G` reaches the maximum offset and `gg` returns to zero.
#[test]
fn markdown_documents_use_existing_vertical_scroll_keys() -> Result<()> {
    let source = (1..=10)
        .map(|index| format!("Paragraph {index}."))
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut view = markdown(source);
    let mut terminal = Terminal::new(TestBackend::new(20, 3))?;
    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    let (offset, max_offset) = markdown_scroll_state(&view);
    assert_eq!(offset, 0);
    assert!(max_offset >= 6);

    view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, 1);
    view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, 0);

    view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, 5);
    view.handle_key_event(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, 0);

    view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, max_offset);
    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, 0);

    Ok(())
}

/// Verifies Markdown headings map to every semantic heading level.
///
/// # Example Under Test
///
/// ```text
/// # One
/// ## Two
/// ### Three
/// #### Four
/// ##### Five
/// ###### Six
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without a fallible API.
/// - H1 through H6 appear in source order.
/// - Every heading retains its text content.
/// - Empty separator paragraphs retain one terminal row between headings.
#[test]
fn markdown_maps_all_heading_levels() {
    let source = concat!(
        "# One\n\n",
        "## Two\n\n",
        "### Three\n\n",
        "#### Four\n\n",
        "##### Five\n\n",
        "###### Six\n",
    );

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            h1("One"),
            h2("Two"),
            h3("Three"),
            h4("Four"),
            h5("Five"),
            h6("Six"),
        ])),
    );
}

/// Verifies Markdown paragraphs retain both line-break forms and Unicode.
///
/// # Example Under Test
///
/// ```text
/// Soft
/// break\
/// hard 界 `code`
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without a fallible API.
/// - Soft and hard breaks become explicit text line boundaries.
/// - Unicode remains intact and inline code uses reverse-video styling.
#[test]
fn markdown_preserves_paragraph_breaks_and_unicode() {
    let source = "Soft\nbreak  \nhard 界 `code`\n";

    assert_eq!(
        markdown(source),
        column([paragraph(Text::from(vec![
            Line::raw("Soft"),
            Line::raw("break"),
            Line::from(vec![
                Span::raw("hard 界 "),
                Span::styled("code", Style::new().add_modifier(Modifier::REVERSED),),
            ]),
        ]))]),
    );
}

/// Verifies Markdown blocks retain a blank terminal row between them.
///
/// # Example Under Test
///
/// ```text
/// One
///
/// Two
/// ```
///
/// # Assertions
///
/// - Parsing inserts one empty semantic paragraph between the blocks.
/// - Rendering retains the empty row between the visible paragraph rows.
#[test]
fn markdown_separates_blocks_with_blank_terminal_rows() -> Result<()> {
    let source = "One\n\nTwo\n";

    assert_eq!(
        markdown(source),
        column([paragraph("One"), paragraph(""), paragraph("Two")]),
    );
    assert_eq!(
        rendered_markdown_lines(source, 8, 3)?,
        ["One     ", "        ", "Two     "],
    );

    Ok(())
}

/// Verifies fenced Markdown code selects highlighting from its first info token.
///
/// # Example Under Test
///
/// ````text
/// ```rust ignored
/// fn main() {}
/// ```
///
/// ```rs
/// let value = true;
/// ```
///
/// ```unknown-language
/// plain
/// ```
/// ````
///
/// # Assertions
///
/// - The first fence uses `rust` rather than the trailing info-string token.
/// - The `rs` alias selects the same bundled Rust grammar.
/// - Unknown languages retain their label and fall back to plain source.
/// - Source-ending newlines remain available to wrapped code rendering.
/// - Empty separator paragraphs retain one terminal row between code blocks.
#[test]
fn markdown_maps_fenced_code_languages_and_fallbacks() {
    let source = concat!(
        "```rust ignored\n",
        "fn main() {}\n",
        "```\n\n",
        "```rs\n",
        "let value = true;\n",
        "```\n\n",
        "```unknown-language\n",
        "plain\n",
        "```\n",
    );

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            code_block("fn main() {}\n").language("rust"),
            code_block("let value = true;\n").language("rs"),
            code_block("plain\n").language("unknown-language"),
        ])),
    );
}

/// Verifies empty fenced and indented Markdown code become plain code blocks.
///
/// # Example Under Test
///
/// ````text
/// ```
/// ```
///
///     plain 界
/// ````
///
/// # Assertions
///
/// - An empty fence produces an empty unlabeled code block.
/// - Indented Unicode source produces an unlabeled plain code block.
/// - Both mappings preserve the code-block builder defaults.
/// - An empty separator paragraph retains one terminal row between blocks.
#[test]
fn markdown_maps_empty_and_indented_code_blocks() {
    let source = "```\n```\n\n    plain 界\n";

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            code_block(""),
            code_block("plain 界\n"),
        ])),
    );
}

/// Verifies Markdown inline syntax produces composable terminal modifiers.
///
/// # Example Under Test
///
/// ```text
/// *outer **bold 界** tail* and **plain** with `code` plus \*escaped\*.
/// ```
///
/// # Assertions
///
/// - Emphasis uses italics and strong text uses bold.
/// - Nested emphasis and strong text combine both modifiers.
/// - Inline code uses reverse video without changing its content.
/// - Escaped delimiters remain literal unstyled text.
/// - Adjacent parser text events coalesce into stable spans.
#[test]
fn markdown_styles_nested_inline_syntax_and_escaped_text() {
    let source = "*outer **bold 界** tail* and **plain** with `code` plus \\*escaped\\*.\n";

    assert_eq!(
        markdown(source),
        column([paragraph(Text::from(Line::from(vec![
            Span::styled("outer ", Style::new().add_modifier(Modifier::ITALIC),),
            Span::styled(
                "bold 界",
                Style::new().add_modifier(Modifier::ITALIC | Modifier::BOLD),
            ),
            Span::styled(" tail", Style::new().add_modifier(Modifier::ITALIC),),
            Span::raw(" and "),
            Span::styled("plain", Style::new().add_modifier(Modifier::BOLD)),
            Span::raw(" with "),
            Span::styled("code", Style::new().add_modifier(Modifier::REVERSED)),
            Span::raw(" plus *escaped*.")
        ])))]),
    );
}

/// Verifies Markdown links remain readable without terminal link interaction.
///
/// # Example Under Test
///
/// ```text
/// Read [the *guide*](https://example.com/guide),
/// [https://example.com](https://example.com), <https://example.org>,
/// and <reader@example.com>, plus [empty]().
/// ```
///
/// # Assertions
///
/// - Link labels are underlined and retain nested emphasis.
/// - A descriptive label is followed by its parenthesized destination.
/// - URL labels and URL autolinks do not duplicate their destinations.
/// - Email autolinks do not expose or duplicate the `mailto:` scheme.
/// - Links with empty destinations do not display empty parentheses.
#[test]
fn markdown_styles_links_and_appends_hidden_destinations() {
    let source = concat!(
        "Read [the *guide*](https://example.com/guide), ",
        "[https://example.com](https://example.com), ",
        "<https://example.org>, and <reader@example.com>, plus [empty]().\n",
    );
    let underline = Style::new().add_modifier(Modifier::UNDERLINED);

    assert_eq!(
        markdown(source),
        column([paragraph(Text::from(Line::from(vec![
            Span::raw("Read "),
            Span::styled("the ", underline),
            Span::styled("guide", underline.add_modifier(Modifier::ITALIC),),
            Span::styled(" (https://example.com/guide)", underline),
            Span::raw(", "),
            Span::styled("https://example.com", underline),
            Span::raw(", "),
            Span::styled("https://example.org", underline),
            Span::raw(", and "),
            Span::styled("reader@example.com", underline),
            Span::raw(", plus "),
            Span::styled("empty", underline),
            Span::raw("."),
        ])))]),
    );
}

/// Verifies Markdown lists retain starts, blocks, nesting, and empty items.
///
/// # Example Under Test
///
/// ```text
/// 3. First
///
///    Second paragraph.
///
///    - Nested bullet
///
///      7. Nested number
///    -
/// 4. Last
/// ```
///
/// # Assertions
///
/// - The outer ordered list starts at three.
/// - Loose item paragraphs remain separate blocks.
/// - Mixed ordered and unordered nesting retains its hierarchy.
/// - Tight-list text becomes paragraphs and empty items remain present.
/// - Empty separator paragraphs retain block spacing inside loose items.
#[test]
fn markdown_preserves_nested_and_mixed_lists() {
    let source = concat!(
        "3. First\n\n",
        "   Second paragraph.\n\n",
        "   - Nested bullet\n\n",
        "     7. Nested number\n",
        "   -\n",
        "4. Last\n",
    );

    assert_eq!(
        markdown(source),
        column([ordered_list([
            list_item(separate_blocks(views![
                paragraph("First"),
                paragraph("Second paragraph."),
                unordered_list([
                    list_item(separate_blocks(views![
                        paragraph("Nested bullet"),
                        ordered_list([list_item([paragraph("Nested number")])]).start(7),
                    ])),
                    list_item(()),
                ]),
            ])),
            list_item([paragraph("Last")]),
        ])
        .start(3)]),
    );
}

/// Verifies Markdown tables map sections, rows, cells, and alignments.
///
/// # Example Under Test
///
/// ```text
/// | Default | Left | Center | Right |
/// | ------- | :--- | :----: | ----: |
/// | a       | b    | c      | d     |
/// ```
///
/// # Assertions
///
/// - Parsing succeeds with the table extension enabled.
/// - Header cells are wrapped in a synthesized semantic header row.
/// - Body rows remain in the semantic table body.
/// - Default, left, center, and right alignments map to semantic cells.
#[test]
fn markdown_maps_table_structure_and_alignment() {
    let source = concat!(
        "| Default | Left | Center | Right |\n",
        "| ------- | :--- | :----: | ----: |\n",
        "| a       | b    | c      | d     |\n",
    );
    let aligned_cells = |values: [&'static str; 4]| {
        [
            table_cell(values[0]).alignment(CellAlignment::Left),
            table_cell(values[1]).alignment(CellAlignment::Left),
            table_cell(values[2]).alignment(CellAlignment::Center),
            table_cell(values[3]).alignment(CellAlignment::Right),
        ]
    };

    assert_eq!(
        markdown(source),
        column([table([
            table_head([table_row(aligned_cells([
                "Default", "Left", "Center", "Right",
            ]))]),
            table_body([table_row(aligned_cells(["a", "b", "c", "d"]))]),
        ])]),
    );
}

/// Verifies nested blockquotes retain semantic children and visible prefixes.
///
/// # Example Under Test
///
/// ```text
/// > Alpha beta gamma
/// >
/// > > Inner
/// ```
///
/// # Assertions
///
/// - Each quote becomes a left-bordered block with readable padding.
/// - The outer border remains visible beside wrapped content.
/// - The nested quote stacks a second border without flattening its child.
/// - The blank row between quote blocks retains the outer quote border.
#[test]
fn markdown_renders_nested_blockquotes_with_wrapped_prefixes() -> Result<()> {
    let source = "> Alpha beta gamma\n>\n> > Inner\n";

    assert_eq!(
        markdown(source),
        column([block_quote(views![
            paragraph("Alpha beta gamma"),
            paragraph(""),
            block_quote(views![paragraph("Inner")]),
        ])]),
    );

    let lines = rendered_markdown_lines(source, 12, 4)?;
    assert!(lines[0].starts_with("│ Alpha beta"));
    assert!(lines[1].starts_with("│ gamma"));
    assert_eq!(lines[2].trim_end(), "│");
    assert!(lines[3].starts_with("│ │ Inner"));

    Ok(())
}

/// Verifies thematic breaks render as width-responsive terminal rules.
///
/// # Example Under Test
///
/// ```text
/// ---
/// ```
///
/// # Assertions
///
/// - A thematic break maps to a dedicated one-row fallback block.
/// - A one-cell terminal renders one horizontal rule glyph without panic.
/// - Wider terminals fill the complete row with rule glyphs.
#[test]
fn markdown_renders_thematic_breaks_at_narrow_widths() -> Result<()> {
    assert_eq!(markdown("---\n"), column([thematic_break()]));
    assert_eq!(rendered_markdown_lines("---\n", 1, 1)?, ["─"]);
    assert_eq!(rendered_markdown_lines("---\n", 6, 1)?, ["──────"]);

    Ok(())
}

/// Verifies Markdown images become descriptive text without image loading.
///
/// # Example Under Test
///
/// ```text
/// ![diagram](https://example.com/diagram.png)
/// ![](local.png)
/// ![caption]()
/// ![]()
/// ```
///
/// # Assertions
///
/// - Alt text and source are both shown when present.
/// - Source-only and alt-only images remain descriptive.
/// - An image with neither value still has a readable label.
/// - Every image maps to text rather than a path-backed image view.
/// - Empty separator paragraphs retain one terminal row between images.
#[test]
fn markdown_maps_images_to_descriptive_text() {
    let source = concat!(
        "![diagram](https://example.com/diagram.png)\n\n",
        "![](local.png)\n\n",
        "![caption]()\n\n",
        "![]()\n",
    );

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            paragraph("Image: diagram (https://example.com/diagram.png)"),
            paragraph("Image: local.png"),
            paragraph("Image: caption"),
            paragraph("Image"),
        ])),
    );
}

/// Verifies raw HTML remains literal and entities follow CommonMark parsing.
///
/// # Example Under Test
///
/// ```text
/// Before <kbd>&amp;</kbd> after.
///
/// <section>
/// block &amp;
/// </section>
///
/// Fish &amp; Chips &copy;
/// ```
///
/// # Assertions
///
/// - Inline HTML tags are retained around decoded CommonMark text.
/// - Block HTML tags, entities, and source line endings remain literal.
/// - Entities in ordinary Markdown text decode to their visible characters.
/// - Following semantic content remains in source order.
/// - Empty separator paragraphs retain one terminal row between blocks.
#[test]
fn markdown_preserves_literal_html_and_entities() {
    let source = concat!(
        "Before <kbd>&amp;</kbd> after.\n\n",
        "<section>\n",
        "block &amp;\n",
        "</section>\n\n",
        "Fish &amp; Chips &copy;\n",
    );

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            paragraph("Before <kbd>&</kbd> after."),
            paragraph(Text::from(vec![
                Line::raw("<section>"),
                Line::raw("block &amp;"),
                Line::raw("</section>"),
                Line::default(),
            ])),
            paragraph("Fish & Chips ©"),
        ])),
    );
}

/// Verifies textual extension events remain readable when encountered.
///
/// # Example Under Test
///
/// ```text
/// - [x] ~~done~~ and $x + y$[^note]
///
/// $$z$$
///
/// [^note]: Detail
/// ```
///
/// # Assertions
///
/// - Unsupported inline presentation drops styling but retains its payload.
/// - Task and footnote events receive readable terminal markers.
/// - Display-math and footnote-definition text stays in source order.
/// - Production parsing remains limited to CommonMark plus tables.
/// - Empty separator paragraphs retain one terminal row between blocks.
#[test]
fn markdown_preserves_text_from_unsupported_parser_events() {
    let source = concat!(
        "- [x] ~~done~~ and $x + y$[^note]\n\n",
        "$$z$$\n\n",
        "[^note]: Detail\n",
    );
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_MATH
        | Options::ENABLE_FOOTNOTES;
    let mut parser = Parser::new_ext(source, options);

    assert_eq!(
        column(parse_blocks(&mut parser, None, MarkdownOptions::default(),)),
        column(separate_blocks(views![
            unordered_list([list_item([paragraph("[x] done and x + y[^note]")])]),
            paragraph("z"),
            paragraph("Detail"),
        ])),
    );
}

/// Verifies semantic blocks and readable fallbacks retain source order.
///
/// # Example Under Test
///
/// ```text
/// # Start
///
/// ---
///
/// ![middle](middle.png)
///
/// <end>
/// ```
///
/// # Assertions
///
/// - The semantic heading remains first.
/// - Rule, image, and raw-HTML fallbacks retain their original order.
/// - Empty separator paragraphs retain one terminal row between fallbacks.
#[test]
fn markdown_preserves_fallback_source_order() {
    let source = "# Start\n\n---\n\n![middle](middle.png)\n\n<end>\n";

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            h1("Start"),
            thematic_break(),
            paragraph("Image: middle (middle.png)"),
            paragraph(Text::from(vec![Line::raw("<end>"), Line::default()])),
        ])),
    );
}

/// Verifies Markdown conversion preserves mixed block source order.
///
/// # Example Under Test
///
/// ```text
/// # 開始
///
/// Before.
///
/// - 中
///
/// ## 終了
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without reordering block types.
/// - Unicode content remains intact in headings and list items.
/// - Empty separator paragraphs retain one terminal row between blocks.
#[test]
fn markdown_preserves_source_order() {
    let source = "# 開始\n\nBefore.\n\n- 中\n\n## 終了\n";

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            h1("開始"),
            paragraph("Before."),
            unordered_list([list_item([paragraph("中")])]),
            h2("終了"),
        ])),
    );
}

/// Verifies empty Markdown produces an empty scrollable document.
///
/// # Example Under Test
///
/// ```text
/// source = ""
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without a fallible API.
/// - The result is an empty semantic column.
#[test]
fn markdown_empty_source_returns_empty_column() {
    assert_eq!(markdown(""), column(()));
}
