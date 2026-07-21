/// Verifies code fixtures preserve fence selection and fallback behavior.
///
/// # Example Under Test
///
/// ```text
/// tests/fixtures/markdown/code.md
/// ```
///
/// # Assertions
///
/// - Fenced code uses only the first info-string token.
/// - The `rust` token and `rs` alias retain highlighted semantic code blocks.
/// - Unknown languages remain labeled while falling back to plain source.
/// - Empty, indented, long, and Unicode code sources remain intact.
/// - Empty separator paragraphs retain one terminal row between code blocks.
#[test]
fn markdown_code_fixture_builds_semantic_views() {
    let actual = markdown(CODE_FIXTURE);
    let expected = column(separated_blocks([
        code_block("fn main() {\n    println!(\"界\");\n}\n").language("rust"),
        code_block("let value = true;\n").language("rs"),
        code_block("plain\n").language("unknown-language"),
        code_block(""),
        code_block("indented 界\n"),
        code_block("abcdefghijklmnopqrstuvwxyz界\n").language("text"),
    ]));
    assert_views_render_equally(&actual, &expected);
}
