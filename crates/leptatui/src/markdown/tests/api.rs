/// Verifies in-memory Markdown readers apply default and custom options.
///
/// # Example Under Test
///
/// ```text
/// markdown("```rust\nfn main() {}\n```")
/// markdown_with_options(source, line numbers enabled)
/// ```
///
/// # Assertions
///
/// - Both in-memory readers return document views without failure.
/// - Default code blocks omit line numbers.
/// - Custom options enable line numbers.
/// - An owned source string is accepted by the option-bearing reader.
#[test]
fn markdown_reader_apis_apply_default_and_custom_options() {
    let source = "```rust\nfn main() {}\n```\n";
    let default = markdown(source);
    assert!(!parsed_code_block_line_numbers(&default));

    let options = MarkdownOptions::default().line_numbers(true);
    let owned_source = source.to_owned();
    let configured = markdown_with_options(owned_source, options);
    assert!(parsed_code_block_line_numbers(&configured));
}

/// Verifies Markdown file readers synchronously load UTF-8 source.
///
/// # Example Under Test
///
/// ```text
/// markdown_file("guide.md")
/// markdown_file_with_options("guide.md", line numbers enabled)
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
    let default_document = markdown(source);
    assert_eq!(
        rendered_view_lines(&default, 80, 20).expect("file document should render"),
        rendered_view_lines(&default_document, 80, 20)
            .expect("in-memory document should render")
    );
    assert_eq!(
        default
            .downcast_ref::<MarkdownView>()
            .expect("file reader should return a Markdown boundary")
            .current_path(),
        fixture_path
    );

    let options = MarkdownOptions::default().line_numbers(true);
    let configured = markdown_file_with_options(&fixture_path, options);
    let expected_configured = markdown_with_options(source, options);
    assert_eq!(
        rendered_view_lines(&configured, 80, 20).expect("configured file should render"),
        rendered_view_lines(&expected_configured, 80, 20)
            .expect("configured in-memory document should render")
    );

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
}

/// Verifies the declarative Markdown element loads through reactive filesystem state.
///
/// # Example Under Test
///
/// ```text
/// <Markdown src="guide.md" line_numbers=false />
/// ```
///
/// # Assertions
///
/// - The selected UTF-8 file renders after its asynchronous operation completes.
/// - Explicitly disabled line numbers omit the code-block gutter.
#[tokio::test(flavor = "current_thread")]
async fn markdown_element_loads_asynchronously_without_line_numbers() {
    let fixture_dir = markdown_fixture_dir("element-reader");
    let fixture_path = fixture_dir.join("guide.md");
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    fs::write(&fixture_path, "```text\nalpha\nbeta\n```\n")
        .expect("Markdown fixture should be written");
    let owner = leptos::prelude::Owner::new();
    let view = owner.with(|| {
        __markdown_element(
            &fixture_path,
            MarkdownOptions::default().line_numbers(false),
            false,
            file!(),
            line!(),
        )
    });

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let rendered = rendered_view_lines(&view, 40, 8)
        .expect("asynchronously loaded Markdown should render")
        .join("\n");
    assert!(rendered.contains("alpha"));
    assert!(!rendered.contains("1 │"));

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
}

/// Verifies declarative Markdown read failures become standard view errors.
///
/// # Example Under Test
///
/// ```text
/// <Markdown src="missing.md" />
/// ```
///
/// # Assertions
///
/// - The fixture directory and source file are created successfully.
/// - The standard error screen is rendered after the asynchronous read fails.
/// - The diagnostic identifies the missing Markdown source path.
/// - The fixture directory is removed after verification.
#[tokio::test(flavor = "current_thread")]
async fn markdown_element_read_failure_renders_view_error() {
    let fixture_dir = markdown_fixture_dir("element-error");
    let missing_path = fixture_dir.join("missing.md");
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    let owner = leptos::prelude::Owner::new();
    let view = owner.with(|| {
        __markdown_element(
            &missing_path,
            MarkdownOptions::default(),
            false,
            file!(),
            line!(),
        )
    });

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let rendered = rendered_view_lines(&view, 120, 20)
        .expect("Markdown error screen should render")
        .join("\n");
    assert!(rendered.contains("Error"));
    assert!(rendered.contains("missing.md"));

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
}

/// Verifies non-editable Markdown elements leave editor shortcuts unhandled.
///
/// # Example Under Test
///
/// ```text
/// <Markdown src="guide.md" />
/// ```
///
/// # Assertions
///
/// - The fixture directory and source file are created successfully.
/// - The default element passes an unmodified `e` key event.
/// - The default element passes an unmodified `r` key event.
/// - The fixture directory is removed after verification.
#[tokio::test(flavor = "current_thread")]
async fn markdown_element_is_not_editable_by_default() {
    let fixture_dir = markdown_fixture_dir("element-not-editable");
    let fixture_path = fixture_dir.join("guide.md");
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    fs::write(&fixture_path, "# Guide\n").expect("Markdown fixture should be written");
    let owner = leptos::prelude::Owner::new();
    let mut view = owner.with(|| {
        __markdown_element(
            &fixture_path,
            MarkdownOptions::default(),
            false,
            file!(),
            line!(),
        )
    });

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(
        view.__dispatch_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))
            .expect("edit key handling should succeed"),
        KeyControl::Pass
    );
    assert_eq!(
        view.__dispatch_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))
            .expect("reload key handling should succeed"),
        KeyControl::Pass
    );

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
}

/// Verifies editable Markdown elements refetch their original source on reload.
///
/// # Example Under Test
///
/// ```text
/// <Markdown src="guide.md" editable=true />
/// ```
///
/// # Assertions
///
/// - The fixture directory and initial source are created successfully.
/// - The initial source is rendered after the first asynchronous read.
/// - The source file is updated successfully.
/// - An unmodified `r` key event is consumed by the element.
/// - The reloaded document contains the changed file contents.
/// - The fixture directory is removed after verification.
#[tokio::test(flavor = "current_thread")]
async fn editable_markdown_element_reloads_its_source() {
    let fixture_dir = markdown_fixture_dir("element-reload");
    let fixture_path = fixture_dir.join("guide.md");
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    fs::write(&fixture_path, "Original contents\n")
        .expect("initial Markdown fixture should be written");
    let owner = leptos::prelude::Owner::new();
    let mut view = owner.with(|| {
        __markdown_element(
            &fixture_path,
            MarkdownOptions::default(),
            true,
            file!(),
            line!(),
        )
    });

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let initial = rendered_view_lines(&view, 80, 8)
        .expect("initial Markdown should render")
        .join("\n");
    assert!(initial.contains("Original contents"));

    fs::write(&fixture_path, "Reloaded contents\n")
        .expect("updated Markdown fixture should be written");
    assert_eq!(
        view.__dispatch_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))
            .expect("reload key handling should succeed"),
        KeyControl::Handled
    );
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let reloaded = rendered_view_lines(&view, 80, 8)
        .expect("reloaded Markdown should render")
        .join("\n");
    assert!(reloaded.contains("Reloaded contents"));
    assert!(!reloaded.contains("Original contents"));

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
}

/// Verifies editable Markdown propagates external-editor setup failures.
///
/// # Example Under Test
///
/// ```text
/// <Markdown src="guide.md" editable=true />
/// ```
///
/// # Assertions
///
/// - The fixture directory and source file are created successfully.
/// - An unmodified `e` key event is consumed by the element.
/// - Editing outside a managed app renders the standard error screen.
/// - The error explains that external editing requires a managed app.
/// - The fixture directory is removed after verification.
#[tokio::test(flavor = "current_thread")]
async fn editable_markdown_element_renders_editor_failures() {
    let fixture_dir = markdown_fixture_dir("element-editor-error");
    let fixture_path = fixture_dir.join("guide.md");
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    fs::write(&fixture_path, "# Guide\n").expect("Markdown fixture should be written");
    let owner = leptos::prelude::Owner::new();
    let mut view = owner.with(|| {
        __markdown_element(
            &fixture_path,
            MarkdownOptions::default(),
            true,
            file!(),
            line!(),
        )
    });

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(
        view.__dispatch_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))
            .expect("edit key handling should succeed"),
        KeyControl::Handled
    );
    tokio::task::yield_now().await;

    let rendered = rendered_view_lines(&view, 120, 20)
        .expect("editor error screen should render")
        .join("\n");
    assert!(rendered.contains("Error"));
    assert!(rendered.contains("external editing requires a managed Leptatui application"));

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
}

/// Verifies view-tree order selects among multiple editable Markdown elements.
///
/// # Example Under Test
///
/// ```text
/// <Div>
///   <Markdown src="first.md" editable=true />
///   <Markdown src="second.md" editable=true />
/// </Div>
/// ```
///
/// # Assertions
///
/// - Both source fixtures are created and updated successfully.
/// - The first editable element consumes an unmodified `r` key event.
/// - The first source is refetched.
/// - The second source remains unchanged until it receives its own reload.
/// - The fixture directory is removed after verification.
#[tokio::test(flavor = "current_thread")]
async fn first_editable_markdown_element_consumes_the_shortcut() {
    let fixture_dir = markdown_fixture_dir("element-order");
    let first_path = fixture_dir.join("first.md");
    let second_path = fixture_dir.join("second.md");
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    fs::write(&first_path, "First original\n").expect("first fixture should be written");
    fs::write(&second_path, "Second original\n").expect("second fixture should be written");
    let owner = leptos::prelude::Owner::new();
    let mut view = owner.with(|| {
        div((
            __markdown_element(
                &first_path,
                MarkdownOptions::default(),
                true,
                file!(),
                line!(),
            ),
            __markdown_element(
                &second_path,
                MarkdownOptions::default(),
                true,
                file!(),
                line!(),
            ),
        ))
        .into_view()
    });

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    fs::write(&first_path, "First reloaded\n").expect("first fixture should be updated");
    fs::write(&second_path, "Second reloaded\n").expect("second fixture should be updated");
    assert_eq!(
        view.__dispatch_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))
            .expect("reload key handling should succeed"),
        KeyControl::Handled
    );
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let rendered = rendered_view_lines(&view, 80, 12)
        .expect("ordered Markdown elements should render")
        .join("\n");
    assert!(rendered.contains("First reloaded"));
    assert!(rendered.contains("Second original"));
    assert!(!rendered.contains("Second reloaded"));

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
        div([paragraph(format!(
            "failed to read Markdown file `{}`: {error}",
            path.display()
        ))])
    };

    let missing_error =
        fs::read_to_string(&missing_path).expect_err("missing fixture should fail to read");
    assert_eq!(missing_error.kind(), io::ErrorKind::NotFound);
    let missing = markdown_file(&missing_path);
    let expected_missing = expected_fallback(&missing_path, &missing_error).into_view();
    assert_eq!(
        rendered_view_lines(&missing, 120, 2).expect("missing fallback should render"),
        rendered_view_lines(&expected_missing, 120, 2)
            .expect("expected missing fallback should render")
    );
    let rendered = rendered_view_lines(&missing, 120, 2)
        .expect("missing-file fallback should render without failure")
        .concat();
    assert!(rendered.contains("failed to read Markdown file"));
    assert!(rendered.contains("missing.md"));

    let directory_error =
        fs::read_to_string(&directory_path).expect_err("directory fixture should fail to read");
    assert_ne!(directory_error.kind(), io::ErrorKind::NotFound);
    let directory = markdown_file(&directory_path);
    let expected_directory = expected_fallback(&directory_path, &directory_error).into_view();
    assert_eq!(
        rendered_view_lines(&directory, 120, 2).expect("directory fallback should render"),
        rendered_view_lines(&expected_directory, 120, 2)
            .expect("expected directory fallback should render")
    );

    let invalid_utf8_error = fs::read_to_string(&invalid_utf8_path)
        .expect_err("invalid UTF-8 fixture should fail to read");
    assert_eq!(invalid_utf8_error.kind(), io::ErrorKind::InvalidData);
    let invalid_utf8 =
        markdown_file_with_options(&invalid_utf8_path, MarkdownOptions::default());
    let expected_invalid_utf8 =
        expected_fallback(&invalid_utf8_path, &invalid_utf8_error).into_view();
    assert_eq!(
        rendered_view_lines(&invalid_utf8, 120, 2)
            .expect("invalid UTF-8 fallback should render"),
        rendered_view_lines(&expected_invalid_utf8, 120, 2)
            .expect("expected invalid UTF-8 fallback should render")
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
    assert_eq!(view, div([paragraph(source)]));
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
