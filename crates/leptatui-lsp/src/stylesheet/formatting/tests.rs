//! Stylesheet layout, trivia preservation, and protocol edit regression tests.

use super::*;

/// User-supplied canonical stylesheet layout.
const EXAMPLE: &str = "stylesheet! {\n    H1 { fg: Color::LightCyan; }\n\n    H2 { fg: Color::Cyan; }\n\n    Button:focus { fg: Color::Yellow; }\n\n    .shell-sell {\n        padding: TuiSpacing::uniform(1);\n\n        &__actions {\n            display: Display::Flex;\n            gap: Axes::all(Length::cells(1.0));\n\n            @media (max-width: 60) {\n                flex_direction: FlexDirection::Column;\n            }\n        }\n    }\n}";

/// Formats a standalone source fragment using default options.
///
/// # Arguments
///
/// * `source` — Rust source containing stylesheet macros.
///
/// # Returns
///
/// A formatted source document.
fn formatted(source: &str) -> String {
    document(source, source, &Options::default(), None)
}

/// Verifies the requested layout and its idempotence.
///
/// # Example Under Test
///
/// The user's headings, focused button, nested shell, and media query.
///
/// # Assertions
///
/// - An unformatted macro produces the exact requested output.
/// - Formatting the canonical output does not change it.
#[test]
fn canonical_example() {
    let source = "stylesheet!{H1{fg:Color::LightCyan;} H2{fg:Color::Cyan;} Button:focus{fg:Color::Yellow;} .shell-sell{padding:TuiSpacing::uniform( 1 ); &__actions{display:Display::Flex;gap:Axes::all(Length::cells(1.0)); @media(max-width:60){flex_direction:FlexDirection::Column;}}}}";
    assert_eq!(
        formatted(source),
        EXAMPLE.replace("stylesheet! {", "stylesheet!{")
    );
    assert_eq!(formatted(EXAMPLE), EXAMPLE);
}

/// Verifies declarations, imports, variables, includes, and mixins retain their order.
///
/// # Example Under Test
///
/// A style module followed by nested rules using local and imported values.
///
/// # Assertions
///
/// - Related declarations stay consecutive and structural groups have blank lines.
/// - Rust calls and stylesheet references receive appropriate spacing.
/// - Qualified macro names and import aliases are formatted.
#[test]
fn modules_and_aliases() {
    let source = "use leptatui::stylesheet as styles;\nfn f() {\n    styles! { @use colors :: palette as p; $a:Color::Red; $b: Color::Blue; @mixin chrome {fg:$ a;bg:p . $b;} H1{ @include chrome; padding:TuiSpacing::new(1,2,3,4); } }\n    leptatui::stylesheet! { H2 {fg:Color::Cyan;} }\n}";
    let after = formatted(source);
    assert!(after.contains("@use colors::palette as p;\n\n        $a: Color::Red;\n        $b: Color::Blue;\n\n        @mixin chrome {\n            fg: $a;\n            bg: p.$b;\n        }"),"{after}");
    assert!(
        after.contains("@include chrome;\n            padding: TuiSpacing::new(1, 2, 3, 4);"),
        "{after}"
    );
    assert!(
        after.contains("leptatui::stylesheet! {\n        H2 { fg: Color::Cyan; }\n    }"),
        "{after}"
    );
    assert_eq!(formatted(&after), after);
}

/// Verifies width limits, media blocks, and editor indentation preferences.
///
/// # Example Under Test
///
/// A long color rule, a single-declaration media block, and a tab-indented block.
///
/// # Assertions
///
/// - Long rules expand, media blocks stay multiline, and tabs are honored.
#[test]
fn width_and_indentation() {
    let source = "stylesheet! { LongCustomComponentName { fg: Color::LightCyan; } H1 { @media(max-width:60){fg:Color::Red;} } }";
    let options = Options {
        width: 40,
        spaces: false,
        tab_size: 2,
        ..Options::default()
    };
    let after = document(source, source, &options, None);
    assert!(
        after.contains("\tLongCustomComponentName {\n\t\tfg: Color::LightCyan;\n\t}"),
        "{after}"
    );
    assert!(
        after.contains("\t\t@media (max-width: 60) {\n\t\t\tfg: Color::Red;"),
        "{after}"
    );
    assert_eq!(document(&after, &after, &options, None), after);
}

/// Verifies comments and multiline literals survive structural and Rust formatting.
///
/// # Example Under Test
///
/// Leading, trailing, inline, and multiline comments plus a multiline raw string.
///
/// # Assertions
///
/// - Comment tokens retain their text and order.
/// - Raw-string contents are unchanged.
/// - Comment-bearing rules expand and the result is idempotent.
#[test]
fn comments_and_literals() {
    let source = "stylesheet! { // heading\nH1{fg:/* accent */ Color::Red;} // trailing\n/* block\n  unchanged */ H2 /* before brace */ { fg: theme_color(r#\"line1\n  line2\"#); bg: Color::Blue; } }";
    let after = formatted(source);
    let before_comments: Vec<_> = printer::comments(source)
        .into_iter()
        .map(|(_, c)| c)
        .collect();
    let after_comments: Vec<_> = printer::comments(&after)
        .into_iter()
        .map(|(_, c)| c)
        .collect();
    assert_eq!(before_comments, after_comments, "{after}");
    assert!(after.contains("r#\"line1\n  line2\"#"), "{after}");
    assert!(after.contains("H1 {\n"), "{after}");
    assert_eq!(formatted(&after), after);
}

/// Verifies incomplete declarations and unbalanced invocations are preserved.
///
/// # Example Under Test
///
/// Missing values, terminators, delimiters, and an unrelated valid macro.
///
/// # Assertions
///
/// - Invalid macros are unchanged even when upstream formatting changed their bodies.
/// - A separate valid macro still formats normally.
#[test]
fn malformed_macros_are_unchanged() {
    for source in [
        "stylesheet! { H1 { fg: } }",
        "stylesheet! { H1 { fg: Color::Red } }",
        "stylesheet! { H1 { fg: Color::Red;",
        "stylesheet! { H1 { fg: make(; } }",
        "stylesheet! { H1 { fg: Color::Red;, } }",
        "stylesheet! { H1 {} }",
        "stylesheet! { @use foo bar; H1 { fg: Color::Red; } }",
        "stylesheet! { . { fg: Color::Red; } }",
        "stylesheet! { @media (max-width:) { H1 { fg: Color::Red; } } }",
    ] {
        assert_eq!(formatted(source), source);
    }
    let source = "stylesheet! { H1 { fg: } }\nstylesheet! { H2{fg:Color::Cyan;} }";
    let upstream = source.replace("fg: }", "fg:    }");
    let after = document(source, &upstream, &Options::default(), None);
    assert!(after.starts_with("stylesheet! { H1 { fg: } }"));
    assert!(after.contains("\n    H2 { fg: Color::Cyan; }\n"));
}

/// Verifies Rust expressions, importance, and comment attachments remain valid.
///
/// # Example Under Test
///
/// Function calls, closures, and a line comment before a declaration terminator.
///
/// # Assertions
///
/// - Rust expressions are formatted and importance is retained.
/// - Line comments cannot swallow a semicolon.
#[test]
fn expression_layout_and_important() {
    let source = "stylesheet! { H1 { fg: Color::Rgb(1,2,3) ! important; padding: {let x=1;TuiSpacing::uniform(x)}; } H2 { fg: Color::Red // keep\n; } }";
    let after = formatted(source);
    assert!(after.contains("Color::Rgb(1, 2, 3) !important;"), "{after}");
    assert!(after.contains("let x = 1;"), "{after}");
    assert!(after.contains("// keep\n"), "{after}");
    assert_eq!(formatted(&after), after);
    let syntax = Syntax::new(&after);
    for m in macros(&syntax) {
        assert!(printable(&syntax, m), "{after}");
    }
}

/// Verifies upstream formatting composes with UTF-16 document edits.
///
/// # Example Under Test
///
/// A source document with an emoji and a Rust spacing edit.
///
/// # Assertions
///
/// - Applying a composed whole-document edit reproduces the result.
/// - Idempotent output returns no edits and overlaps are rejected.
#[test]
fn composes_protocol_edits() {
    let source = "// 😀\nfn f(){stylesheet! { H1{fg:Color::Red;} }}";
    let upstream = source.replace("fn f(){", "fn f() {\n    ");
    let after = document(source, &upstream, &Options::default(), None);
    assert_eq!(apply(source, &edits(source, &after)), Some(after.clone()));
    assert_eq!(edits(&after, &after), json!([]));
    let overlapping = json!([
        {"range":super::super::range(source,0..8),"newText":"a"},
        {"range":super::super::range(source,0..1),"newText":"b"}
    ]);
    assert!(apply(source, &overlapping).is_none());
}

/// Verifies project preferences and explicit editor indentation compose correctly.
///
/// # Example Under Test
///
/// A temporary Rust 2021 package with a 72-column, hard-tab rustfmt configuration.
///
/// # Assertions
///
/// - Width and edition come from the project.
/// - Explicit editor indentation overrides project indentation.
#[test]
fn project_preferences() {
    let root = std::env::temp_dir().join(format!(
        "leptatui-formatter-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"),"[package]\nname = \"formatter-fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n[workspace]\n").unwrap();
    std::fs::write(
        root.join("rustfmt.toml"),
        "max_width = 72\nhard_tabs = true\ntab_spaces = 8\n",
    )
    .unwrap();
    let path = root.join("src/lib.rs");
    std::fs::write(&path, "").unwrap();
    let defaults = expressions::options(Some(&path), &json!({}));
    assert_eq!(defaults.width, 72);
    assert_eq!(defaults.edition, "2021");
    assert_eq!(defaults.tab_size, 8);
    assert!(!defaults.spaces);
    let explicit = expressions::options(Some(&path), &json!({"tabSize":2,"insertSpaces":true}));
    assert_eq!(explicit.tab_size, 2);
    assert!(explicit.spaces);
    assert!(!root.join("Cargo.lock").exists());
    std::fs::remove_dir_all(root).unwrap();
}

/// Verifies line comments and Windows line endings survive printing.
///
/// # Example Under Test
///
/// A directive comment before its semicolon and a CRLF stylesheet.
///
/// # Assertions
///
/// - Directive terminators remain outside comments.
/// - Generated code keeps CRLF and formatting is idempotent.
#[test]
fn directive_comments_and_crlf() {
    let source =
        "stylesheet! { @mixin chrome { fg: Color::Red; } H1 { @include chrome // keep\n; } }";
    let after = formatted(source);
    assert!(after.contains("// keep\n"), "{after}");
    assert_eq!(formatted(&after), after);
    let source = "stylesheet! {\r\nH1 { fg: Color::Red; }\r\n}";
    let after = formatted(source);
    assert_eq!(after, "stylesheet! {\r\n    H1 { fg: Color::Red; }\r\n}");
    assert_eq!(formatted(&after), after);
}
