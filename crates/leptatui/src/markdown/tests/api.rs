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
