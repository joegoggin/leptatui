/// Verifies percent-encoded local Markdown paths load decoded filenames.
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

/// Verifies explicit suffixed headings do not collide with duplicate slugs.
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
