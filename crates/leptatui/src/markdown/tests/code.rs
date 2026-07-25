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
        div(separate_blocks(views![
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
        div(separate_blocks(views![
            code_block(""),
            code_block("plain 界\n"),
        ])),
    );
}
