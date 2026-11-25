//! Static class and ID associations between stylesheet and view macros.

use crate::stylesheet::syntax::{Block, Kind, Syntax};
use serde_json::{Value, json};
use std::{collections::HashMap, ops::Range};

/// A statically authored selector or view attribute name.
struct Occurrence {
    /// Resolved name, including the parent of a BEM suffix.
    name: String,
    /// Class or ID namespace.
    kind: &'static str,
    /// Source name range.
    span: Range<usize>,
    /// Parent prefix omitted from this source occurrence.
    parent: String,
    /// Whether the occurrence is in a stylesheet selector.
    definition: bool,
}

impl Occurrence {
    /// Matches a name or its immediately preceding selector sigil.
    ///
    /// # Arguments
    ///
    /// * `text` — Source document with unsaved edits applied.
    /// * `cursor` — Request byte offset at a UTF-8 boundary.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the request selects this occurrence.
    fn contains(&self, text: &str, cursor: usize) -> bool {
        self.span.start <= cursor && cursor <= self.span.end
            || self.definition
                && cursor + 1 == self.span.start
                && matches!(text.as_bytes().get(cursor), Some(b'.' | b'#' | b'&'))
    }
}

/// Returns literal class/ID attribute contents, skipping embedded Rust expressions.
///
/// # Arguments
///
/// * `syntax` — Tokenized source and recoverable delimiter pairs.
/// * `m` — Containing macro invocation.
///
/// # Returns
///
/// A list of literal attribute namespaces and source content ranges.
fn attributes(
    syntax: &Syntax<'_>,
    m: &crate::stylesheet::syntax::Macro,
) -> Vec<(&'static str, Range<usize>, bool)> {
    let mut result = Vec::new();
    let mut i = m.open + 1;
    while i < m.end {
        if matches!(syntax.at(i), "class" | "id") && syntax.at(i + 1) == "=" {
            let start = i + 2;
            let literal = if syntax.at(start) == "{" && syntax.end(start, m.end) == start + 2 {
                start + 1
            } else {
                start
            };
            if let Some(t) = syntax.tokens.get(literal)
                && t.literal
                && let Some(first) = t.text.find('"')
                && let Some(last) = t.text.rfind('"')
                && first < last
            {
                result.push((
                    if syntax.at(i) == "id" { "id" } else { "class" },
                    t.span.start + first + 1..t.span.start + last,
                    t.text.starts_with('r'),
                ));
            }
        }
        if matches!(syntax.at(i), "{" | "(" | "[") {
            i = syntax.end(i, m.end);
        }
        i += 1;
    }
    result
}

/// Recovers class/ID names from structural rule headers and expanded BEM suffixes.
///
/// # Arguments
///
/// * `syntax` — Tokenized source and recoverable delimiter pairs.
/// * `block` — Stylesheet block to inspect.
/// * `parent` — Containing selector or block context.
/// * `output` — Destination for collected results.
fn rules(syntax: &Syntax<'_>, block: &Block, parent: &str, output: &mut Vec<Occurrence>) {
    for s in &block.statements {
        let mut child_parent = parent.to_owned();
        if s.kind == Kind::Rule {
            let start = s.head.start;
            let sigil = syntax.at(start);
            if matches!(sigil, "." | "#" | "&") && syntax.at(start + 1) != ":" {
                let end = (start + 1..s.head.end)
                    .find(|&i| syntax.at(i) == ":")
                    .unwrap_or(s.head.end);
                if start + 1 < end {
                    let span = syntax.tokens[start + 1].span.start..syntax.tokens[end - 1].span.end;
                    let authored = syntax.tokens[start + 1..end]
                        .iter()
                        .map(|t| t.text)
                        .collect::<String>();
                    let base = if sigil == "&" { parent } else { "" };
                    if sigil != "&"
                        || !base.is_empty()
                            && (authored.starts_with("__") || authored.starts_with("--"))
                    {
                        let name = format!("{base}{authored}");
                        output.push(Occurrence {
                            name: name.clone(),
                            kind: if sigil == "#" { "id" } else { "class" },
                            span,
                            parent: base.into(),
                            definition: true,
                        });
                        child_parent = if sigil == "#" { String::new() } else { name };
                    }
                }
            } else if sigil != "&" {
                child_parent.clear();
            }
        }
        if let Some(body) = &s.body {
            rules(syntax, body, &child_parent, output);
        }
    }
}

/// Finds the terminal class eligible for a BEM suffix at a nested cursor position.
///
/// # Arguments
///
/// * `syntax` — Tokenized source and recoverable delimiter pairs.
/// * `block` — Stylesheet block to inspect.
/// * `cursor` — Request byte offset at a UTF-8 boundary.
/// * `parent` — Current terminal class, or an empty string for non-class selectors.
///
/// # Returns
///
/// A resolved parent class name, or an empty string when a suffix is invalid.
fn class_at(syntax: &Syntax<'_>, block: &Block, cursor: usize, parent: &str) -> String {
    for s in &block.statements {
        if let Some(body) = &s.body
            && body.span.start <= cursor
            && cursor <= body.span.end
        {
            let mut name = parent.to_owned();
            if s.kind == Kind::Rule {
                let sigil = syntax.at(s.head.start);
                let authored = syntax.tokens[s.head.start + 1..s.head.end]
                    .iter()
                    .map(|t| t.text)
                    .collect::<String>();
                name = if sigil == "." {
                    authored
                } else if sigil == "&" && !authored.starts_with(':') {
                    format!("{parent}{authored}")
                } else {
                    String::new()
                };
            }
            return class_at(syntax, body, cursor, &name);
        }
    }
    parent.into()
}

/// Decodes a literal while retaining the source range of each resulting character.
///
/// # Arguments
///
/// * `text` — Source document with unsaved edits applied.
/// * `span` — Literal contents excluding delimiters.
/// * `raw` — Whether Rust escape processing is disabled.
///
/// # Returns
///
/// An optional sequence of decoded characters and original byte ranges.
fn decoded(text: &str, span: Range<usize>, raw: bool) -> Option<Vec<(char, Range<usize>)>> {
    let mut result = Vec::new();
    let mut valid = true;
    let mut append =
        |range: Range<usize>, value: Result<char, rustc_lexer::unescape::EscapeError>| {
            if let Ok(c) = value {
                result.push((c, span.start + range.start..span.start + range.end));
            } else {
                valid = false;
            }
        };
    if raw {
        rustc_lexer::unescape::unescape_raw_str(&text[span.clone()], &mut append);
    } else {
        rustc_lexer::unescape::unescape_str(&text[span.clone()], &mut append);
    }
    valid.then_some(result)
}

/// Indexes literal names while excluding dynamic attributes and Rust expressions.
///
/// # Arguments
///
/// * `text` — Source document with unsaved edits applied.
///
/// # Returns
///
/// A list of static class and ID occurrences.
fn occurrences(text: &str) -> Vec<Occurrence> {
    let syntax = Syntax::new(text);
    let mut result = Vec::new();
    for m in &syntax.macros {
        if m.view {
            for (kind, span, raw) in attributes(&syntax, m) {
                let Some(chars) = decoded(text, span, raw) else {
                    continue;
                };
                let mut name = String::new();
                let mut word = 0..0;
                for (c, span) in chars.into_iter().chain([(' ', 0..0)]) {
                    if c.is_whitespace() {
                        if !name.is_empty() {
                            result.push(Occurrence {
                                name: std::mem::take(&mut name),
                                kind,
                                span: word.clone(),
                                parent: String::new(),
                                definition: false,
                            });
                        }
                    } else {
                        if name.is_empty() {
                            word.start = span.start;
                        }
                        word.end = span.end;
                        name.push(c);
                    }
                }
            }
        } else {
            rules(&syntax, &syntax.parse(m), "", &mut result);
        }
    }
    result
}

/// Supplies workspace selector and literal attribute completions.
///
/// # Arguments
///
/// * `documents` — Workspace source documents, including unsaved overlays.
/// * `uri` — Requested source document URI.
/// * `cursor` — Request byte offset at a UTF-8 boundary.
///
/// # Returns
///
/// An optional list when the position expects a class or ID name.
pub(crate) fn complete(
    documents: &HashMap<String, String>,
    uri: &str,
    cursor: usize,
) -> Option<Vec<Value>> {
    let text = documents.get(uri)?;
    let syntax = Syntax::new(text);
    let mut word = syntax.word(cursor);
    let mut kind = None;
    let mut parent = String::new();
    let mut view = false;
    let mut literal_prefix = None;
    if let Some(m) = syntax.stylesheet(cursor) {
        if syntax.excluded(cursor) {
            return None;
        }
        let tree = syntax.parse(m);
        let (block, statement) = tree.context(cursor);
        if !block.rules || statement.is_some_and(|s| s.kind != Kind::Rule) {
            return None;
        }
        let previous = syntax
            .tokens
            .iter()
            .rev()
            .find(|t| t.span.end <= word.start);
        kind = previous.and_then(|t| match t.text {
            "." => Some("class"),
            "#" => Some("id"),
            "&" => Some("class"),
            _ => None,
        });
        if previous.is_some_and(|t| t.text == "&") {
            parent = class_at(&syntax, &tree, cursor, "");
            if parent.is_empty() {
                return Some(Vec::new());
            }
        }
    } else {
        for m in syntax.macros.iter().filter(|m| m.view) {
            if let Some((k, span, raw)) = attributes(&syntax, m)
                .into_iter()
                .find(|(_, r, _)| r.start <= cursor && cursor <= r.end)
            {
                if let Some(found) = occurrences(text)
                    .into_iter()
                    .find(|o| !o.definition && o.span.start <= cursor && cursor <= o.span.end)
                {
                    word = found.span;
                }
                if let Some(chars) = decoded(text, span, raw) {
                    literal_prefix = Some(
                        chars
                            .into_iter()
                            .filter(|(_, r)| r.start >= word.start && r.end <= cursor)
                            .map(|(c, _)| c)
                            .collect::<String>(),
                    );
                }
                kind = Some(k);
                view = true;
                break;
            }
        }
    }
    let kind = kind?;
    let prefix = literal_prefix
        .as_deref()
        .unwrap_or(&text[word.start..cursor]);
    let mut names: Vec<_> = documents
        .values()
        .flat_map(|s| occurrences(s))
        .filter(|o| o.kind == kind)
        .filter_map(|o| o.name.strip_prefix(&parent).map(str::to_owned))
        .filter(|n| n.starts_with(prefix))
        .collect();
    names.sort();
    names.dedup();
    Some(names.into_iter().map(|name|json!({"label":name,"kind":12,"detail":format!("Workspace {kind}"),"textEdit":{"range":range(text,word.clone()),"newText":name},"insertTextFormat":1,"data":{"leptatuiStylesheet":true},"sortText":if view {"0"} else {"1"}})).collect())
}

/// Converts a source range to UTF-16 protocol coordinates.
///
/// # Arguments
///
/// * `text` — Source document with unsaved edits applied.
/// * `span` — Source byte range.
///
/// # Returns
///
/// A UTF-16 protocol range.
fn range(text: &str, span: Range<usize>) -> Value {
    let pos = |n: usize| {
        let p = &text[..n];
        json!({"line":p.bytes().filter(|b|*b==b'\n').count(),"character":p.rsplit('\n').next().unwrap_or("").encode_utf16().count()})
    };
    json!({"start":pos(span.start),"end":pos(span.end)})
}

/// Answers static selector navigation and rename requests across workspace overlays.
///
/// # Arguments
///
/// * `documents` — Workspace source documents, including unsaved overlays.
/// * `uri` — Requested source document URI.
/// * `cursor` — Request byte offset at a UTF-8 boundary.
/// * `method` — Requested LSP method.
/// * `params` — Original request parameters.
///
/// # Returns
///
/// An optional protocol result for the request.
pub(crate) fn semantic(
    documents: &HashMap<String, String>,
    uri: &str,
    cursor: usize,
    method: &str,
    params: &Value,
) -> Option<Value> {
    let text = documents.get(uri)?;
    let local = occurrences(text);
    let selected = local.iter().find(|o| o.contains(text, cursor))?;
    let all: Vec<_> = documents
        .iter()
        .flat_map(|(uri, text)| occurrences(text).into_iter().map(move |o| (uri, text, o)))
        .collect();
    let matching: Vec<_> = all
        .iter()
        .filter(|(_, _, o)| o.name == selected.name && o.kind == selected.kind)
        .collect();
    match method {
        "textDocument/hover" => Some(
            json!({"contents":{"kind":"markdown","value":format!("Stylesheet {} `{}`",selected.kind,selected.name)}}),
        ),
        "textDocument/definition" | "textDocument/references" => Some(Value::Array(
            matching
                .iter()
                .filter(|(_, _, o)| {
                    if method.ends_with("definition") {
                        o.definition
                    } else {
                        !o.definition || params["context"]["includeDeclaration"] == true
                    }
                })
                .map(|(uri, text, o)| json!({"uri":uri,"range":range(text,o.span.clone())}))
                .collect(),
        )),
        "textDocument/prepareRename" => Some(
            json!({"range":range(documents.get(uri)?,selected.span.clone()),"placeholder":selected.name}),
        ),
        "textDocument/rename" => {
            let name = params["newName"].as_str()?;
            if name.is_empty()
                || name.starts_with('-')
                || !name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return None;
            }
            if !selected.parent.is_empty() && !name.starts_with(&selected.parent) {
                return None;
            }
            let mut changes: HashMap<String, Vec<Value>> = HashMap::new();
            for (uri, text, o) in &all {
                if o.kind != selected.kind {
                    continue;
                }
                let dependent = o
                    .name
                    .strip_prefix(&selected.name)
                    .is_some_and(|s| s.starts_with("__") || s.starts_with("--"));
                if o.name != selected.name && !dependent {
                    continue;
                }
                // A parent rename changes an implicit BEM prefix automatically.
                if dependent && o.parent.starts_with(&selected.name) {
                    continue;
                }
                let full = format!("{name}{}", &o.name[selected.name.len()..]);
                let replacement = full.strip_prefix(&o.parent)?;
                changes
                    .entry((*uri).clone())
                    .or_default()
                    .push(json!({"range":range(text,o.span.clone()),"newText":replacement}));
            }
            Some(json!({"changes":changes}))
        }
        _ => None,
    }
}

/// Rejects selector renames that cannot preserve authored BEM structure.
///
/// # Arguments
///
/// * `documents` — Workspace source documents, including unsaved overlays.
/// * `uri` — Requested source document URI.
/// * `cursor` — Request byte offset at a UTF-8 boundary.
/// * `name` — Proposed replacement symbol or selector name.
///
/// # Returns
///
/// An optional reason the requested selector rename cannot preserve source syntax.
pub(crate) fn rename_error(
    documents: &HashMap<String, String>,
    uri: &str,
    cursor: usize,
    name: &str,
) -> Option<String> {
    let text = documents.get(uri)?;
    let local = occurrences(text);
    let selected = local.iter().find(|o| o.contains(text, cursor))?;
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Some("selector names must contain identifiers separated by hyphens".into());
    }
    let ident = |value: &str| syn::parse_str::<syn::Ident>(value).is_ok();
    for o in documents
        .values()
        .flat_map(|text| occurrences(text))
        .filter(|o| o.definition && o.kind == selected.kind)
    {
        let dependent = o
            .name
            .strip_prefix(&selected.name)
            .is_some_and(|s| s.starts_with("__") || s.starts_with("--"));
        if o.name != selected.name && !dependent {
            continue;
        }
        if dependent && o.parent.starts_with(&selected.name) {
            continue;
        }
        let full = format!("{name}{}", &o.name[selected.name.len()..]);
        let Some(authored) = full.strip_prefix(&o.parent) else {
            return Some(
                "rename would require changing a BEM parent; rename the parent class separately"
                    .into(),
            );
        };
        let valid = if o.parent.is_empty() {
            authored.split('-').all(ident)
        } else if authored.starts_with("__") {
            ident(authored)
        } else if let Some(rest) = authored.strip_prefix("--") {
            rest.split('-').all(ident)
        } else {
            false
        };
        if !valid {
            return Some("rename cannot be represented by the existing selector syntax".into());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    //! Selector association and BEM edit regressions.
    use super::*;

    /// Verifies nested Rust expressions do not introduce selector occurrences.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// stylesheet! { .panel { fg: person.color; } }
    /// view! { <Div class="panel">{let class="other"; class}</Div> }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Only the authored stylesheet selector and literal view class are indexed.
    #[test]
    fn ignores_rust_values_and_dynamic_attributes() {
        let text = "stylesheet! { .panel { fg: person.color; } } view! { <Div class=\"panel\">{let class=\"other\"; class}</Div> }";
        let names = occurrences(text);
        assert_eq!(names.len(), 2);
        assert!(names.iter().all(|o| o.name == "panel"));
    }

    /// Verifies a BEM rename updates expanded literal names while retaining suffix syntax.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// stylesheet! { .panel { &__title { &--big { fg: Color::Red; } } } }
    /// view! { <Text class="panel__title panel__title--big" /> }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Renaming the parent requires three edits and leaves implicit suffixes untouched.
    /// - Moving the BEM child to another parent is rejected.
    #[test]
    fn bem_parent_rename_preserves_implicit_suffixes() {
        let text = "stylesheet! { .panel { &__title { &--big { fg: Color::Red; } } } } view! { <Text class=\"panel__title panel__title--big\" /> }";
        let docs = HashMap::from([("file:///test.rs".into(), text.into())]);
        let cursor = text.find("panel").unwrap() + 2;
        let edit = semantic(
            &docs,
            "file:///test.rs",
            cursor,
            "textDocument/rename",
            &json!({"newName":"card"}),
        )
        .unwrap();
        assert_eq!(
            edit["changes"]["file:///test.rs"].as_array().unwrap().len(),
            3
        );
        assert!(
            rename_error(
                &docs,
                "file:///test.rs",
                text.find("__title").unwrap() + 2,
                "card__title"
            )
            .is_some()
        );
    }

    /// Verifies Unicode positions and raw literal attributes retain UTF-16 ranges.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// // 🐱
    /// view! { <Div class={r"panel selected"} /> }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Both static class names are found and protocol ranges preserve their source.
    #[test]
    fn raw_and_braced_literals_have_source_ranges() {
        let text = "// 🐱\nview! { <Div class={r\"panel selected\"} /> }";
        let found = occurrences(text);
        assert_eq!(found.len(), 2);
        for o in found {
            let r = range(text, o.span.clone());
            assert_eq!(r["start"]["line"], 1);
            assert_eq!(&text[o.span], o.name);
        }
    }
    /// Verifies escaped class names retain editable source spans.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// view! { <Div class="pan\\u{65}l other" /> }
    /// ```
    ///
    /// # Assertions
    ///
    /// - The escape decodes to `panel` and its source range includes the full escape.
    #[test]
    fn escaped_names_preserve_source_ranges() {
        let text = r#"view! { <Div class="pan\u{65}l other" /> }"#;
        let found = occurrences(text);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "panel");
        assert_eq!(&text[found[0].span.clone()], r"pan\u{65}l");
    }
}
