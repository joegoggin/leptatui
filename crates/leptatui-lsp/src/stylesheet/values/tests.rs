//! Expected-value completion behavior and catalog coverage.

use super::*;

/// Returns completions at a marked source position.
///
/// # Arguments
///
/// * `marked` — Source containing one cursor marker.
/// * `snippets` — Client snippet support.
///
/// # Returns
///
/// A list of expected-value items.
fn at(marked: &str, snippets: bool) -> Vec<Value> {
    let cursor = marked.find('~').unwrap();
    complete(&marked.replace('~', ""), cursor, snippets)
}

/// Verifies all compiler properties have parseable default expressions.
///
/// # Example Under Test
///
/// Every shared property category and its complete value catalog.
///
/// # Assertions
///
/// - Each property offers values at an empty declaration.
/// - Defaults parse as Rust expressions and contain no snippet markers.
#[test]
fn every_property_has_valid_templates() {
    for &(property, category, _, _) in properties::PROPERTIES {
        let items = at(&format!("stylesheet! {{ H1 {{ {property}: ~ }} }}"), false);
        assert!(!items.is_empty(), "{property}: {category}");
        for item in items {
            let expression = item["textEdit"]["newText"].as_str().unwrap();
            assert!(!expression.contains('$'), "{expression}");
            syn::parse_str::<syn::Expr>(expression)
                .unwrap_or_else(|e| panic!("{property}: {expression}: {e}"));
        }
    }
}

/// Verifies bare and qualified prefixes preserve authored paths and suffix ranges.
///
/// # Example Under Test
///
/// ```rust,ignore
/// stylesheet! { H1 { fg: re~d; } }
/// stylesheet! { H1 { fg: palette::Color::Li~ghtRed; } }
/// ```
///
/// # Assertions
///
/// - Member matching ignores case.
/// - Qualified paths are retained and the full suffix is replaced.
/// - Constructor snippets contain numbered defaults.
#[test]
fn prefixes_paths_and_snippets() {
    let items = at("stylesheet! { H1 { fg: re~d; } }", true);
    let red = items.iter().find(|i| i["label"] == "Color::Red").unwrap();
    assert_eq!(red["textEdit"]["newText"], "Color::Red");
    assert_eq!(red["filterText"], "Red");
    let items = at(
        "stylesheet! { H1 { fg: palette::Color::Li~ghtRed; } }",
        false,
    );
    let red = items
        .iter()
        .find(|i| i["label"] == "Color::LightRed")
        .unwrap();
    assert_eq!(red["textEdit"]["newText"], "palette::Color::LightRed");
    assert_eq!(red["textEdit"]["range"]["end"]["character"], 47);
    let items = at("stylesheet! { H1 { fg: Color::Rg~; } }", true);
    assert_eq!(
        items[0]["textEdit"]["newText"],
        "Color::Rgb(${1:255}, ${2:255}, ${3:255})"
    );
}

/// Verifies value completion recovers nested syntax and subsequent flag operands.
///
/// # Example Under Test
///
/// Nested media, mixin and incomplete selector bodies, and `Modifier::BOLD |`.
///
/// # Assertions
///
/// - These positions suggest the expected category.
/// - Flag completion leaves preceding operands outside the edit range.
#[test]
fn nesting_and_flags() {
    for source in [
        "stylesheet! { H1 { fg: ~",
        "stylesheet! { .panel { @media (max-width: 80) { fg: ~ } } }",
        "stylesheet! { @mixin chrome { foreground: ~ } }",
        "use leptatui::stylesheet as styles; fn f() { styles! { H1 { &:focus { bg: ~ } } } }",
    ] {
        assert!(
            at(source, false).iter().any(|i| i["label"] == "Color::Red"),
            "{source}"
        );
    }
    let source = "stylesheet! { H1 { modifier: Modifier::BOLD | un~; } }";
    let items = at(source, false);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["textEdit"]["newText"], "Modifier::UNDERLINED");
    assert_eq!(
        items[0]["textEdit"]["range"]["start"]["character"],
        source.find("un~").unwrap()
    );
}

/// Verifies embedded Rust and non-value positions keep upstream completion.
///
/// # Example Under Test
///
/// Calls, structs, strings, comments, variable initializers, and unknown properties.
///
/// # Assertions
///
/// - No expected-value templates are injected into these contexts.
#[test]
fn excludes_other_contexts() {
    for source in [
        "stylesheet! { H1 { ~ } }",
        "stylesheet! { H1 { fg: make(~) } }",
        "stylesheet! { H1 { padding: TuiSpacing { left: ~ } } }",
        "stylesheet! { H1 { fg: person.~ } }",
        "stylesheet! { H1 { fg: $ac~ } }",
        "stylesheet! { H1 { fg: color + ~ } }",
        "stylesheet! { H1 { fg: color !~ } }",
        "stylesheet! { H1 { fg: \"co~lor\" } }",
        "stylesheet! { H1 { /* fg: ~ */ } }",
        "stylesheet! { $accent: ~; }",
        "stylesheet! { H1 { unknown: ~ } }",
        "fn main() { let fg = ~; }",
    ] {
        assert!(at(source, true).is_empty(), "{source}");
    }
}

/// Verifies Unicode preceding the cursor produces UTF-16 protocol edits.
///
/// # Example Under Test
///
/// A declaration following an emoji comment.
///
/// # Assertions
///
/// - Edit positions count UTF-16 code units rather than UTF-8 bytes.
#[test]
fn unicode_edit_ranges() {
    let source = "stylesheet! { H1 { /* 😀 */ fg: Re~; } }";
    let item = at(source, false)
        .into_iter()
        .find(|i| i["label"] == "Color::Red")
        .unwrap();
    let start = source[..source.find("Re~").unwrap()].encode_utf16().count();
    assert_eq!(item["textEdit"]["range"]["start"]["character"], start);
    assert_eq!(item["textEdit"]["range"]["end"]["character"], start + 2);
}

/// Verifies runtime unit variants remain represented in the editor catalog.
///
/// # Example Under Test
///
/// The public box-model, flex, alignment, position, and grid enum definitions.
///
/// # Assertions
///
/// - Every unit variant of a property enum has a corresponding template.
/// - Overflow variants are wrapped in axes rather than offered as bare values.
#[test]
fn enum_catalog_tracks_runtime_definitions() {
    for source in [
        include_str!("../../../../leptatui/src/style/layout/box_model.rs"),
        include_str!("../../../../leptatui/src/style/layout/flex.rs"),
        include_str!("../../../../leptatui/src/style/layout/alignment.rs"),
        include_str!("../../../../leptatui/src/style/layout/position.rs"),
        include_str!("../../../../leptatui/src/style/layout/grid.rs"),
    ] {
        for item in syn::parse_file(source).unwrap().items {
            let syn::Item::Enum(item) = item else {
                continue;
            };
            let ty = item.ident.to_string();
            if !properties::PROPERTIES.iter().any(|p| p.1 == ty) {
                continue;
            }
            for variant in item.variants {
                if !matches!(variant.fields, syn::Fields::Unit) {
                    continue;
                }
                let name = format!("{ty}::{}", variant.ident);
                let expression = if ty == "Overflow" {
                    format!("Axes::all({name})")
                } else {
                    name
                };
                assert!(
                    templates(&ty).iter().any(|t| t.expression == expression),
                    "{expression}"
                );
            }
        }
    }
}

/// Verifies completions omit generated crate prefixes throughout expressions.
///
/// # Example Under Test
///
/// Empty color values, bare names, nested constructors, and authored crate paths.
///
/// # Assertions
///
/// - Colors and constructor argument types use unqualified names.
/// - Explicitly authored crate paths remain intact.
/// - Both snippet and plain-text clients receive the same path behavior.
#[test]
fn inserted_values_use_unqualified_types() {
    for snippets in [false, true] {
        for source in [
            "stylesheet! { H1 { fg: ~ } }",
            "stylesheet! { H1 { fg: Cy~ } }",
        ] {
            let item = at(source, snippets)
                .into_iter()
                .find(|item| item["label"] == "Color::Cyan")
                .unwrap();
            assert_eq!(item["textEdit"]["newText"], "Color::Cyan");
        }
        let item = at(
            "stylesheet! { H1 { fg: ::leptatui::Color::Cy~ } }",
            snippets,
        )
        .into_iter()
        .find(|item| item["label"] == "Color::Cyan")
        .unwrap();
        assert_eq!(item["textEdit"]["newText"], "::leptatui::Color::Cyan");
    }
    for (source, expected) in [
        (
            "stylesheet! { H1 { size: La~ } }",
            "LayoutSize::new(Dimension::Length(Length::percent(100.0)), Dimension::Auto)",
        ),
        (
            "stylesheet! { H1 { size: ui::LayoutSize::ne~ } }",
            "ui::LayoutSize::new(Dimension::Length(Length::percent(100.0)), Dimension::Auto)",
        ),
    ] {
        let item = at(source, false)
            .into_iter()
            .find(|item| item["label"] == "LayoutSize::new")
            .unwrap();
        assert_eq!(item["textEdit"]["newText"], expected);
    }
}
