/// Verifies percent-encoded local Markdown paths load decoded filenames.
///
/// # Example Under Test
///
/// ```text
/// reader.md: [Guide](Target%20Guide.md)
/// Target Guide.md: # Encoded target
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - Activating the link navigates to the decoded `Target Guide.md` path.
/// - The linked Markdown document is loaded and rendered.
#[test]
fn markdown_file_links_decode_percent_encoded_paths() -> Result<()> {
    let fixture_dir = markdown_fixture_dir("encoded-link-path");
    let markdown_path = fixture_dir.join("reader.md");
    let target_path = fixture_dir.join("Target Guide.md");
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    fs::write(&markdown_path, "[Guide](Target%20Guide.md)")
        .expect("source Markdown fixture should be written");
    fs::write(&target_path, "# Encoded target")
        .expect("target Markdown fixture should be written");

    let mut document = markdown_file(&markdown_path);
    document.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
    document.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;

    let boundary = document
        .downcast_ref::<MarkdownView>()
        .expect("file reader should return a Markdown boundary");
    assert_eq!(boundary.current_path(), target_path);
    let rendered = rendered_view_lines(&document, 80, 3)?.concat();
    assert!(rendered.contains("Encoded target"));

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
    Ok(())
}

/// Verifies accepted Markdown navigation marks the source link visited.
///
/// # Example Under Test
///
/// ```text
/// root.md: [Guide](guide.md)
/// Enter, then H to return through Markdown history
/// ```
///
/// # Assertions
///
/// - Link activation navigates to `guide.md` inside the active session.
/// - Markdown history returns to the source document.
/// - The restored link renders magenta and underlined.
#[test]
fn markdown_navigation_marks_restored_link_visited_for_session() -> Result<()> {
    let fixture_dir = markdown_fixture_dir("visited-navigation");
    let root_path = fixture_dir.join("root.md");
    let target_path = fixture_dir.join("guide.md");
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    fs::write(&root_path, "[Guide](guide.md)")
        .expect("source Markdown fixture should be written");
    fs::write(&target_path, "# Guide").expect("target Markdown fixture should be written");

    let registry = crate::view::VisitedLinkRegistry::new();
    let mut document = markdown_file(&root_path);
    let mut terminal = Terminal::new(TestBackend::new(8, 1))?;
    registry.with(|| -> Result<()> {
        document.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
        document.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;
        assert_eq!(
            document
                .downcast_ref::<MarkdownView>()
                .expect("file reader should return a Markdown boundary")
                .current_path(),
            target_path
        );

        document.handle_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE))?;
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            render_result = document.render(&mut ctx);
        })?;
        render_result
    })?;

    for cell in &terminal.backend().buffer().content()[..5] {
        assert_eq!(cell.fg, Color::Magenta);
        assert!(cell.modifier.contains(Modifier::UNDERLINED));
    }

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
    Ok(())
}

/// Verifies percent-encoded non-Markdown paths resolve to decoded filenames.
///
/// # Example Under Test
///
/// ```text
/// [Guide](User%20Guide.pdf)
/// ```
///
/// # Assertions
///
/// - The focused link is a filesystem target.
/// - Its resolved path ends in `User Guide.pdf`, not `User%20Guide.pdf`.
#[test]
fn markdown_local_links_decode_percent_encoded_paths() -> Result<()> {
    let fixture_dir = markdown_fixture_dir("encoded-local-link-path");
    let markdown_path = fixture_dir.join("reader.md");
    let target_path = fixture_dir.join("User Guide.pdf");
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    fs::write(&markdown_path, "[Guide](User%20Guide.pdf)")
        .expect("source Markdown fixture should be written");

    let mut document = markdown_file(&markdown_path);
    document.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;

    assert_eq!(
        document.__focused_link_target(),
        Some(LinkTarget::Path(target_path))
    );

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
    Ok(())
}

/// Verifies explicit suffixed headings do not collide with duplicate slugs.
///
/// # Example Under Test
///
/// ```text
/// [Third heading](target.md#foo-2)
///
/// # Foo
/// # Foo-1
/// # Foo
/// ```
///
/// # Assertions
///
/// - Activating the link navigates to `target.md`.
/// - The generated `foo-2` anchor selects the third heading.
#[test]
fn markdown_heading_slugs_avoid_explicit_suffix_collisions() -> Result<()> {
    let fixture_dir = markdown_fixture_dir("heading-slug-collisions");
    let root_path = fixture_dir.join("root.md");
    let target_path = fixture_dir.join("target.md");
    fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
    fs::write(&root_path, "[Third heading](target.md#foo-2)\n")
        .expect("root Markdown fixture should be written");
    fs::write(
        &target_path,
        "# Foo\n\nFirst.\n\n# Foo-1\n\nSecond.\n\n# Foo\n\nThird.\n",
    )
    .expect("target Markdown fixture should be written");

    let mut document = markdown_file(&root_path);
    document.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
    document.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;
    let rendered = rendered_view_lines(&document, 20, 3)?.concat();

    let boundary = document
        .downcast_ref::<MarkdownView>()
        .expect("file reader should return a Markdown boundary");
    assert_eq!(boundary.current_path(), target_path);
    assert!(rendered.contains("Third."), "rendered text: {rendered:?}");

    fs::remove_dir_all(&fixture_dir).expect("fixture directory should be removed");
    Ok(())
}
