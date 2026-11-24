//! Identifier indexing and UTF-16 source locations for semantic rename queries.
//!
//! Rust's tokenizer excludes comments and string contents from rename candidates.
//! Resolution and visibility remain rust-analyzer's responsibility.

use proc_macro2::{TokenStream, TokenTree};
use serde_json::{Value, json};
use std::{collections::HashMap, fs, path::Path};
use url::Url;

/// A source identifier and its original LSP range.
#[derive(Clone)]
pub(crate) struct Identifier {
    /// Spelling, without a raw-identifier prefix.
    pub(crate) name: String,
    /// UTF-16 start and end positions.
    pub(crate) range: Value,
}

/// Converts a tokenizer position into an LSP UTF-16 position.
///
/// # Arguments
///
/// * `text` — Complete Rust document.
/// * `line` — One-based tokenizer line.
/// * `column` — Zero-based character column.
///
/// # Returns
///
/// A JSON LSP position.
fn position(text: &str, line: usize, column: usize) -> Value {
    let character = text
        .lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .chars()
        .take(column)
        .map(char::len_utf16)
        .sum::<usize>();
    json!({"line": line.saturating_sub(1), "character": character})
}

/// Collects identifiers recursively while excluding literals and comments.
///
/// # Arguments
///
/// * `tokens` — Tokenized Rust source.
/// * `text` — Original source for UTF-16 conversion.
/// * `output` — Identifier collection to extend.
fn collect(tokens: TokenStream, text: &str, output: &mut Vec<Identifier>) {
    for token in tokens {
        match token {
            TokenTree::Ident(ident) => {
                let span = ident.span();
                let spelling = ident.to_string();
                output.push(Identifier {
                    name: spelling.strip_prefix("r#").unwrap_or(&spelling).to_owned(),
                    range: json!({
                        "start": position(text, span.start().line, span.start().column),
                        "end": position(text, span.end().line, span.end().column),
                    }),
                });
            }
            TokenTree::Group(group) => collect(group.stream(), text, output),
            _ => {}
        }
    }
}

/// Returns identifiers from a balanced Rust document.
///
/// # Arguments
///
/// * `text` — Source document, including unsaved changes.
///
/// # Returns
///
/// A vector of identifiers; an unbalanced document returns an empty vector.
pub(crate) fn identifiers(text: &str) -> Vec<Identifier> {
    let mut result = Vec::new();
    if let Ok(tokens) = text.parse() {
        collect(tokens, text, &mut result);
    }
    result
}

/// Reads Rust documents beneath a workspace root, including open document overlays.
///
/// # Arguments
///
/// * `root` — Workspace directory.
/// * `documents` — Document map extended with files not already open.
pub(crate) fn documents(root: &Path, documents: &mut HashMap<String, String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|ty| ty.is_symlink()) {
            continue;
        }
        if path.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some("target" | ".git" | "node_modules" | "vendor")
            ) {
                self::documents(&path, documents);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && let Ok(uri) = Url::from_file_path(&path)
            && let Ok(text) = fs::read_to_string(&path)
        {
            documents.entry(uri.to_string()).or_insert(text);
        }
    }
}

/// Returns a byte offset for an LSP UTF-16 position.
///
/// # Arguments
///
/// * `text` — Source document.
/// * `position` — LSP line and character.
///
/// # Returns
///
/// An optional byte offset; invalid positions return [`None`].
pub(crate) fn offset(text: &str, position: &Value) -> Option<usize> {
    let line = position["line"].as_u64()? as usize;
    let character = position["character"].as_u64()? as usize;
    let start = if line == 0 {
        0
    } else {
        text.match_indices('\n').nth(line - 1)?.0 + 1
    };
    let mut units = 0;
    for (index, ch) in text[start..].char_indices() {
        if units == character {
            return Some(start + index);
        }
        if ch == '\n' {
            return None;
        }
        units += ch.len_utf16();
    }
    (units == character).then_some(text.len())
}

/// A component declaration or prop parameter represented by generated symbols.
pub(crate) struct Symbol {
    /// Source declaration identifier.
    pub(crate) ident: Identifier,
    /// Whether generated type names share the declaration's name prefix.
    pub(crate) component: bool,
}

/// Finds source declarations whose generated symbols must be renamed together.
///
/// # Arguments
///
/// * `text` — Rust source containing component declarations.
///
/// # Returns
///
/// A vector of component and prop declaration locations.
pub(crate) fn component_symbols(text: &str) -> Vec<Symbol> {
    use syn::visit::Visit;
    /// Visitor retaining only annotated component signatures.
    struct Components<'a> {
        /// Original document used for UTF-16 mapping.
        text: &'a str,
        /// Component and prop declarations discovered so far.
        symbols: Vec<Symbol>,
    }
    impl Components<'_> {
        /// Records a declaration at its original source span.
        ///
        /// # Arguments
        ///
        /// * `ident` — Source declaration name.
        /// * `component` — Whether derived type names belong to this symbol.
        fn push(&mut self, ident: &syn::Ident, component: bool) {
            let span = ident.span();
            self.symbols.push(Symbol {
                ident: Identifier {
                    name: ident.to_string().trim_start_matches("r#").to_owned(),
                    range: json!({
                        "start": position(self.text, span.start().line, span.start().column),
                        "end": position(self.text, span.end().line, span.end().column),
                    }),
                },
                component,
            });
        }
    }
    impl<'ast> Visit<'ast> for Components<'_> {
        fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
            if item.attrs.iter().any(|attr| {
                attr.path()
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "component")
            }) {
                self.push(&item.sig.ident, true);
                for arg in &item.sig.inputs {
                    if let syn::FnArg::Typed(arg) = arg
                        && let syn::Pat::Ident(pattern) = arg.pat.as_ref()
                    {
                        self.push(&pattern.ident, false);
                    }
                }
            }
            syn::visit::visit_item_fn(self, item);
        }
    }
    let mut visitor = Components {
        text,
        symbols: Vec::new(),
    };
    if let Ok(file) = syn::parse_file(text) {
        visitor.visit_file(&file);
    }
    visitor.symbols
}

#[cfg(test)]
mod tests {
    //! Source mapping regressions for non-ASCII Rust documents.

    use super::*;

    /// Verifies identifier locations survive Unicode, raw identifiers, and literals.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// let café = "🙂 title"; r#type(café); // title
    /// ```
    ///
    /// # Assertions
    ///
    /// - Literal and comment contents are not indexed as identifiers.
    /// - Every reported UTF-16 range selects its original identifier.
    #[test]
    fn identifiers_preserve_utf16_ranges() {
        let text = "let café = \"🙂 title\"; r#type(café); // title\n";
        let identifiers = identifiers(text);
        assert!(!identifiers.iter().any(|ident| ident.name == "title"));
        for ident in identifiers {
            let start = offset(text, &ident.range["start"]).unwrap();
            let end = offset(text, &ident.range["end"]).unwrap();
            assert_eq!(text[start..end].trim_start_matches("r#"), ident.name);
        }
    }

    /// Verifies declaration discovery excludes ordinary functions and Rust locals.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// #[leptatui::component] fn Panel(title: String) -> impl IntoView { view!{} }
    /// fn other(title: String) {}
    /// ```
    ///
    /// # Assertions
    ///
    /// - Only the annotated component and its prop parameter are discovered.
    #[test]
    fn discovers_component_declarations_only() {
        let text = "#[leptatui::component] fn Panel(title: String) -> impl IntoView { view!{} } fn other(title: String) {}";
        let symbols = component_symbols(text);
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].ident.name, "Panel");
        assert!(symbols[0].component);
        assert_eq!(symbols[1].ident.name, "title");
        assert!(!symbols[1].component);
    }
}
