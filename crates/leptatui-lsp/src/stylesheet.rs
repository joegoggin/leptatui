//! Recovering stylesheet contexts, vocabulary completions, and local symbols.
//!
//! # Modules
//!
//! - [`syntax`] — Recovering Rust token groups and stylesheet blocks.
//! - [`mod@imports`] — Rust-resolved style-module exports and refactoring.
//! - [`proxy`] — Protocol request and completion lifecycle.
//! - [`diagnostics`] — Source diagnostics and upstream result merging.
//! - `properties` — Property metadata shared with compiler validation.
//! - [`formatting`] — Stylesheet printing and document-formatting composition.
//! - [`values`] — Expected-value contexts, documentation, and expression templates.

#[path = "../../leptatui-macros/src/stylesheet/properties.rs"]
mod properties;

use serde_json::{Value, json};
use std::ops::Range;

mod diagnostics;
pub(crate) mod formatting;
pub(crate) mod imports;
mod proxy;
pub(crate) mod syntax;
pub(crate) mod values;
use syntax::{Kind, Syntax};

/// Converts byte offsets to UTF-16 protocol ranges.
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
    let position = |offset: usize| {
        let prefix = &text[..offset];
        json!({"line":prefix.bytes().filter(|b| *b == b'\n').count(),
            "character":prefix.rsplit('\n').next().unwrap_or("").encode_utf16().count()})
    };
    json!({"start":position(span.start),"end":position(span.end)})
}

/// Returns context-sensitive completions inside a stylesheet, including unfinished input.
///
/// # Arguments
///
/// * `text` — Current source overlay.
/// * `cursor` — Completion byte offset.
/// * `snippets` — Client snippet capability.
///
/// # Returns
///
/// An optional list of stylesheet completions; Rust expressions use upstream results.
pub(crate) fn complete(text: &str, cursor: usize, snippets: bool) -> Option<Vec<Value>> {
    let syntax = Syntax::new(text);
    let m = syntax.stylesheet(cursor)?;
    if syntax.excluded(cursor) {
        return None;
    }
    let tree = syntax.parse(m);
    let (block, statement) = tree.context(cursor);
    let word = syntax.word(cursor);
    let prefix = &text[word.start..cursor];
    let previous: Vec<_> = syntax
        .tokens
        .iter()
        .enumerate()
        .filter(|(i, t)| *i > m.open && *i < m.end && t.span.end <= word.start)
        .collect();
    let last = previous.last().map_or("", |(_, t)| t.text);
    let head = statement.map(|s| s.head.clone()).unwrap_or(0..0);
    let role = statement.map(|s| s.kind);
    if role == Some(Kind::Property)
        && head.clone().any(|i| {
            matches!(syntax.at(i), "{" | "(" | "[")
                && syntax.tokens[i].span.end <= cursor
                && cursor
                    <= syntax
                        .tokens
                        .get(syntax.end(i, head.end))
                        .map_or(text.len(), |t| t.span.start)
        })
    {
        return None;
    }
    let slot = if snippets { "$0" } else { "" };
    let mut candidates: Vec<(String, String, String, u64)> = Vec::new();
    let mut add = |name: &str, insert: String, detail: &str, kind: u64| {
        candidates.push((name.into(), insert, detail.into(), kind))
    };
    let in_value = role == Some(Kind::Property)
        && previous
            .iter()
            .any(|(i, t)| head.contains(i) && t.text == ":");
    let colon = last == ":"
        && previous
            .iter()
            .rev()
            .nth(1)
            .is_none_or(|(_, t)| t.text != ":");
    if last == "!" && in_value {
        add(
            "important",
            "important".into(),
            "Overrides normal inline and stylesheet declarations",
            14,
        );
    } else if last == "$" || role == Some(Kind::Include) {
        if last == "."
            || previous
                .iter()
                .rev()
                .nth(1)
                .is_some_and(|(_, t)| t.text == ".")
        {
            return Some(Vec::new());
        }
        let kind = if role == Some(Kind::Include) {
            "mixin"
        } else {
            "variable"
        };
        for symbol in symbols(text)
            .into_iter()
            .filter(|s| s.scope == m.open && s.declaration && s.kind == kind)
        {
            add(
                &symbol.name,
                symbol.name.clone(),
                &format!("Stylesheet {kind}"),
                6,
            );
        }
        for import in imports(text).into_iter().filter(|i| i.scope == m.open) {
            if kind == "mixin" {
                add(
                    &import.alias,
                    format!("{}.", import.alias),
                    "Imported stylesheet module",
                    9,
                );
            }
        }
    } else if role == Some(Kind::Use) {
        if last == "as" {
            return Some(Vec::new());
        }
        return None;
    } else if in_value
        || role == Some(Kind::Variable)
            && previous
                .iter()
                .any(|(i, t)| head.contains(i) && t.text == ":")
    {
        return None;
    } else if role == Some(Kind::Media) {
        for feature in ["min-width", "max-width", "min-height", "max-height"] {
            let insert = if text[word.end..].trim_start().starts_with(':') {
                feature.into()
            } else {
                format!("{feature}: {slot}")
            };
            add(feature, insert, "Terminal viewport size in cells", 10);
        }
        add("and", "and".into(), "Combine viewport conditions", 14);
    } else if last == "@" {
        let directives = if !block.declarations {
            vec!["use", "mixin", "media"]
        } else if !block.rules {
            vec!["include"]
        } else {
            vec!["include", "media"]
        };
        for name in directives {
            add(name, name.into(), "Stylesheet directive", 14);
        }
    } else if colon || last == "&" {
        for name in ["focus", "active", "insert", "visual", "visited"] {
            add(
                name,
                if last == "&" {
                    format!(":{name}")
                } else {
                    name.into()
                },
                "Selector pseudo-class",
                13,
            );
        }
    } else if last == "." || last == "#" || prefix.starts_with("__") || prefix.starts_with("--") {
        // Workspace selector names are supplied by selectors::complete.
    } else {
        if block.declarations {
            for &(name, kind, method, _) in properties::PROPERTIES {
                let insert = if text[word.end..].trim_start().starts_with(':') {
                    name.into()
                } else {
                    format!(
                        "{name}: {slot}{}",
                        if text[word.end..].trim_start().starts_with(';') {
                            ""
                        } else {
                            ";"
                        }
                    )
                };
                add(
                    name,
                    insert,
                    &format!("{} — {}", property_type(kind), method),
                    10,
                );
            }
        }
        if block.rules {
            for name in [
                "Block",
                "Div",
                "Form",
                "Text",
                "H1",
                "H2",
                "H3",
                "H4",
                "H5",
                "H6",
                "Paragraph",
                "Link",
                "A",
                "Routes",
                "Route",
                "ParentRoute",
                "Outlet",
                "Markdown",
                "CodeBlock",
                "OrderedList",
                "UnorderedList",
                "ListItem",
                "Table",
                "TableHead",
                "TableBody",
                "TableRow",
                "TableCell",
                "Button",
                "Input",
                "TextArea",
                "Image",
                "ProgressBar",
            ] {
                let insert = if text[word.end..].trim_start().starts_with(['{', ':']) {
                    name.into()
                } else {
                    format!("{name} {{ {slot} }}")
                };
                add(
                    name,
                    insert,
                    "Terminal type selector (custom names are also supported)",
                    7,
                );
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    Some(
        candidates
            .into_iter()
            .filter(|(name, _, _, _)| name.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|(name, insert, detail, kind)| {
                let documentation = properties::PROPERTIES
                    .iter()
                    .find(|p| p.0 == name)
                    .map(|p| values::property_documentation(p.0, p.1, p.2))
                    .unwrap_or_else(|| detail.clone());
                json!({
                    "label": name,
                    "kind": kind,
                    "sortText": format!("{}_{}", if kind == 10 { 0 } else { 1 }, name),
                    "detail": detail,
                    "documentation": {"kind": "markdown", "value": documentation},
                    "textEdit": {"range": range(text, word.clone()), "newText": insert},
                    "insertTextFormat": if snippets { 2 } else { 1 },
                    "data": {"leptatuiStylesheet": true}
                })
            })
            .collect(),
    )
}

/// Returns the Rust type corresponding to a compiler value category.
///
/// # Arguments
///
/// * `kind` — Compiler value category.
///
/// # Returns
///
/// A Rust type description for the value category.
fn property_type(kind: &str) -> &str {
    match kind {
        "Spacing" => "TuiSpacing",
        "Overflow" => "Axes<Overflow>",
        "Size" => "TuiSize",
        "Number" => "f32",
        "GridTemplateTracks" => "Vec<GridTemplateTrack>",
        "GridAutoTracks" => "Vec<GridTrackSize>",
        "LayoutSize" => "LayoutSize<Dimension>",
        "LengthAutoEdges" => "Edges<LengthAuto>",
        "Gap" => "Axes<Length>",
        other => other,
    }
}

/// Adds stylesheet variable and module candidates to a Rust value completion response.
///
/// # Arguments
///
/// * `text` — Source document with unsaved edits applied.
/// * `cursor` — Request byte offset at a UTF-8 boundary.
///
/// # Returns
///
/// A list of stylesheet names to merge with Rust completions.
pub(crate) fn rust_completions(text: &str, cursor: usize) -> Vec<Value> {
    let syntax = Syntax::new(text);
    let Some(m) = syntax.stylesheet(cursor) else {
        return Vec::new();
    };
    if syntax.excluded(cursor) {
        return Vec::new();
    }
    let tree = syntax.parse(m);
    let (_, statement) = tree.context(cursor);
    let Some(s) = statement else {
        return Vec::new();
    };
    if s.kind != Kind::Property {
        return Vec::new();
    }
    let word = syntax.word(cursor);
    if syntax.tokens[s.head.start + 2..s.head.end]
        .iter()
        .any(|t| t.span.end <= word.start)
    {
        return Vec::new();
    }
    let prefix = &text[word.start..cursor];
    let mut candidates = Vec::new();
    for symbol in symbols(text)
        .into_iter()
        .filter(|s| s.scope == m.open && s.declaration && s.kind == "variable")
    {
        let label = format!("${}", symbol.name);
        if prefix.is_empty() || label.starts_with(prefix) || symbol.name.starts_with(prefix) {
            candidates.push((label.clone(), label, "Stylesheet variable"));
        }
    }
    for import in imports(text)
        .into_iter()
        .filter(|i| i.scope == m.open && i.alias.starts_with(prefix))
    {
        candidates.push((
            import.alias.clone(),
            format!("{}.", import.alias),
            "Imported stylesheet module",
        ));
    }
    candidates.into_iter().map(|(label,insert,detail)|json!({"label":label,"kind":6,"detail":detail,"textEdit":{"range":range(text,word.clone()),"newText":insert},"insertTextFormat":1,"data":{"leptatuiStylesheet":true}})).collect()
}

/// Completes custom component names as open terminal type selectors.
///
/// # Arguments
///
/// * `documents` — Workspace source documents, including unsaved overlays.
/// * `uri` — Requested source document URI.
/// * `cursor` — Request byte offset at a UTF-8 boundary.
/// * `snippets` — Whether the client supports snippet tab stops.
///
/// # Returns
///
/// A list of matching custom type-selector completions.
pub(crate) fn type_completions(
    documents: &std::collections::HashMap<String, String>,
    uri: &str,
    cursor: usize,
    snippets: bool,
) -> Vec<Value> {
    let Some(text) = documents.get(uri) else {
        return Vec::new();
    };
    let syntax = Syntax::new(text);
    let Some(m) = syntax.stylesheet(cursor) else {
        return Vec::new();
    };
    if syntax.excluded(cursor) {
        return Vec::new();
    }
    let tree = syntax.parse(m);
    let (block, statement) = tree.context(cursor);
    let word = syntax.word(cursor);
    if !block.rules
        || statement.is_some_and(|s| {
            s.kind != Kind::Rule
                || s.head.start < s.head.end && syntax.tokens[s.head.start].span.start < word.start
        })
    {
        return Vec::new();
    }
    let mut names: Vec<_> = documents
        .values()
        .flat_map(|text| crate::source::component_symbols(text))
        .filter(|s| s.component)
        .map(|s| s.ident.name)
        .filter(|name| {
            name.to_lowercase()
                .starts_with(&text[word.start..cursor].to_lowercase())
        })
        .collect();
    names.sort();
    names.dedup();
    names.into_iter().map(|name| {
        let insert=if text[word.end..].trim_start().starts_with(['{',':']) {name.clone()} else {format!("{name} {{ {} }}",if snippets {"$0"} else {""})};
        json!({"label":name,"kind":7,"detail":"Custom terminal type selector","textEdit":{"range":range(text,word.clone()),"newText":insert},"insertTextFormat":if snippets {2} else {1},"data":{"leptatuiStylesheet":true}})
    }).collect()
}

/// A source occurrence of a stylesheet variable, mixin, or import alias.
#[derive(Clone)]
pub(crate) struct Symbol {
    /// Symbol spelling without punctuation.
    pub name: String,
    /// Namespace: variable, mixin, or module.
    pub kind: &'static str,
    /// Opening token of its stylesheet invocation.
    pub scope: usize,
    /// Authored name range.
    pub span: Range<usize>,
    /// Whether this occurrence introduces the symbol.
    pub declaration: bool,
    /// Imported namespace, if the member is qualified.
    pub alias: Option<String>,
}

impl Symbol {
    /// Matches a stylesheet symbol name or its variable sigil.
    ///
    /// # Arguments
    ///
    /// * `text` — Source document with unsaved edits applied.
    /// * `cursor` — Request byte offset at a UTF-8 boundary.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the request selects this symbol.
    pub(crate) fn contains(&self, text: &str, cursor: usize) -> bool {
        self.span.start <= cursor && cursor <= self.span.end
            || self.kind == "variable"
                && cursor + 1 == self.span.start
                && text.as_bytes().get(cursor) == Some(&b'$')
    }
}

/// A stylesheet import resolved through rust-analyzer's Rust name resolver.
#[derive(Clone)]
pub(crate) struct Import {
    /// Containing macro opening token.
    pub scope: usize,
    /// Namespace visible in this stylesheet.
    pub alias: String,
    /// Source range of the final Rust path segment.
    pub path: Range<usize>,
    /// Whether an explicit `as` namespace was authored.
    pub explicit: bool,
}

/// Collects stylesheet imports without guessing which Rust function a path names.
///
/// # Arguments
///
/// * `text` — Source document with unsaved edits applied.
///
/// # Returns
///
/// A list of authored stylesheet imports.
pub(crate) fn imports(text: &str) -> Vec<Import> {
    let syntax = Syntax::new(text);
    let mut result = Vec::new();
    for m in syntax.macros.iter().filter(|m| !m.view) {
        let tree = syntax.parse(m);
        for s in tree.statements.iter().filter(|s| s.kind == Kind::Use) {
            let end = (s.head.start + 2..s.head.end)
                .find(|&i| syntax.at(i) == "as")
                .unwrap_or(s.head.end);
            if let Some(token) = syntax.tokens[s.head.start + 2..end]
                .iter()
                .rev()
                .find(|t| t.ident)
            {
                let alias = if end < s.head.end {
                    syntax.at(end + 1)
                } else {
                    token.text
                };
                result.push(Import {
                    scope: m.open,
                    alias: alias.into(),
                    path: token.span.clone(),
                    explicit: end < s.head.end,
                });
            }
        }
    }
    result
}

/// Indexes declarations and authored references using stylesheet statement boundaries.
///
/// # Arguments
///
/// * `text` — Source document with unsaved edits applied.
///
/// # Returns
///
/// A list of scoped declarations and references.
pub(crate) fn symbols(text: &str) -> Vec<Symbol> {
    let syntax = Syntax::new(text);
    let mut result = Vec::new();
    for m in syntax.macros.iter().filter(|m| !m.view) {
        let tree = syntax.parse(m);
        let mut statements = Vec::new();
        tree.walk(&mut statements);
        for s in statements {
            let name_index = match s.kind {
                Kind::Variable => Some(s.head.start + 1),
                Kind::Mixin => Some(s.head.start + 2),
                _ => None,
            };
            if let Some(i) = name_index
                && let Some(t) = syntax.tokens.get(i)
                && t.ident
            {
                result.push(Symbol {
                    name: t.text.into(),
                    kind: if s.kind == Kind::Variable {
                        "variable"
                    } else {
                        "mixin"
                    },
                    scope: m.open,
                    span: t.span.clone(),
                    declaration: true,
                    alias: None,
                });
            }
            if s.kind == Kind::Use {
                let as_index = s.head.clone().find(|&i| syntax.at(i) == "as");
                let i = as_index.map_or(s.head.end.saturating_sub(1), |i| i + 1);
                if let Some(t) = syntax.tokens.get(i)
                    && t.ident
                {
                    result.push(Symbol {
                        name: t.text.into(),
                        kind: "module",
                        scope: m.open,
                        span: t.span.clone(),
                        declaration: true,
                        alias: None,
                    });
                }
            }
            let value_start = match s.kind {
                Kind::Property => s.head.start + 2,
                Kind::Include => s.head.start + 2,
                _ => continue,
            };
            let mut i = value_start;
            while i < s.head.end {
                if syntax.tokens[i].ident
                    && syntax.at(i + 1) == "."
                    && (s.kind == Kind::Include || syntax.at(i + 2) == "$")
                {
                    let member = i + 2 + usize::from(syntax.at(i + 2) == "$");
                    result.push(Symbol {
                        name: syntax.at(i).into(),
                        kind: "module",
                        scope: m.open,
                        span: syntax.tokens[i].span.clone(),
                        declaration: false,
                        alias: None,
                    });
                    if member < s.head.end && syntax.tokens[member].ident {
                        result.push(Symbol {
                            name: syntax.at(member).into(),
                            kind: if s.kind == Kind::Include {
                                "mixin"
                            } else {
                                "variable"
                            },
                            scope: m.open,
                            span: syntax.tokens[member].span.clone(),
                            declaration: false,
                            alias: Some(syntax.at(i).into()),
                        });
                    }
                    break;
                }
                let member = if syntax.at(i) == "$" {
                    Some(i + 1)
                } else if s.kind == Kind::Include && i == value_start {
                    Some(i)
                } else {
                    None
                };
                if let Some(member) = member
                    && member < s.head.end
                    && syntax.tokens[member].ident
                {
                    result.push(Symbol {
                        name: syntax.at(member).into(),
                        kind: if s.kind == Kind::Include {
                            "mixin"
                        } else {
                            "variable"
                        },
                        scope: m.open,
                        span: syntax.tokens[member].span.clone(),
                        declaration: false,
                        alias: None,
                    });
                    break;
                }
                // Rust value expressions cannot contain stylesheet variable references.
                i = if matches!(syntax.at(i), "{" | "(" | "[") {
                    syntax.end(i, s.head.end)
                } else {
                    i
                };
                i += 1;
            }
        }
    }
    result
}

/// Answers semantic requests for local stylesheet symbols and property hover.
///
/// # Arguments
///
/// * `text` — Current document overlay.
/// * `uri` — Document URI.
/// * `cursor` — Request byte offset.
/// * `method` — LSP method.
/// * `params` — Original request parameters.
///
/// # Returns
///
/// An optional result; unresolved Rust requests remain upstream.
pub(crate) fn semantic(
    text: &str,
    uri: &str,
    cursor: usize,
    method: &str,
    params: &Value,
) -> Option<Value> {
    if method == "textDocument/hover"
        && let Some(hover) = language_hover(text, cursor)
    {
        return Some(hover);
    }
    let symbols = symbols(text);
    let selected = symbols.iter().find(|s| s.contains(text, cursor))?;
    if selected.alias.is_some() {
        return None;
    }
    let matching: Vec<_> = symbols
        .iter()
        .filter(|s| {
            s.scope == selected.scope
                && s.kind == selected.kind
                && s.name == selected.name
                && s.alias == selected.alias
        })
        .collect();
    let declaration = matching.iter().find(|s| s.declaration);
    let location = |s: &&Symbol| json!({"uri":uri,"range":range(text,s.span.clone())});
    match method {
        "textDocument/hover" => Some(
            json!({"contents":{"kind":"markdown","value":format!("Stylesheet {} `{}`", selected.kind,selected.name)},"range":range(text,selected.span.clone())}),
        ),
        "textDocument/definition" => Some(declaration.map(location).unwrap_or(Value::Null)),
        "textDocument/prepareRename" => declaration.map(
            |_| json!({"range":range(text,selected.span.clone()),"placeholder":selected.name}),
        ),
        "textDocument/references" => Some(Value::Array(
            matching
                .iter()
                .filter(|s| !s.declaration || params["context"]["includeDeclaration"] == true)
                .map(location)
                .collect(),
        )),
        "textDocument/rename" => {
            let name = params["newName"].as_str()?;
            if declaration.is_none() || syn::parse_str::<syn::Ident>(name).is_err() {
                return None;
            }
            let edits: Vec<_> = matching
                .iter()
                .map(|s| {
                    let implicit=s.kind=="module" && s.declaration && imports(text).iter().any(|i|i.scope==s.scope && i.alias==s.name && !i.explicit);
                    if implicit {json!({"range":range(text,s.span.end..s.span.end),"newText":format!(" as {name}")})}
                    else {json!({"range":range(text,s.span.clone()),"newText":name})}
                })
                .collect();
            Some(json!({"changes":{uri:edits}}))
        }
        _ => None,
    }
}

/// Returns documentation for stylesheet grammar tokens without intercepting Rust expressions.
///
/// # Arguments
///
/// * `text` — Source document with unsaved edits applied.
/// * `cursor` — Request byte offset at a UTF-8 boundary.
///
/// # Returns
///
/// An optional documentation result for a stylesheet grammar token.
fn language_hover(text: &str, cursor: usize) -> Option<Value> {
    let syntax = Syntax::new(text);
    let m = syntax.stylesheet(cursor)?;
    if syntax.excluded(cursor) {
        return None;
    }
    let tree = syntax.parse(m);
    let (_, statement) = tree.context(cursor);
    let s = statement?;
    let word = syntax.word(cursor);
    let name = &text[word.clone()];
    let token = syntax
        .tokens
        .iter()
        .position(|t| t.span.start == word.start)?;
    let detail = if s.kind == Kind::Property && token == s.head.start {
        let (_, kind, builder, _) = properties::PROPERTIES.iter().find(|p| p.0 == name)?;
        values::property_documentation(name, kind, builder)
    } else if name == "important" && token > 0 && syntax.at(token - 1) == "!" {
        "`!important` gives this declaration priority over normal inline and stylesheet styles."
            .into()
    } else if s.kind == Kind::Rule && token > 0 && syntax.at(token - 1) == ":" {
        match name {
            "focus" => "Matches the focused view.",
            "active" => "Matches an active view.",
            "insert" => "Matches an input or text area in Vim Insert mode.",
            "visual" => "Matches an input or text area in Vim Visual or Visual Line mode.",
            "visited" => "Matches a link opened during this app session.",
            _ => return None,
        }
        .into()
    } else if s.kind == Kind::Rule && token == s.head.start && syntax.tokens[token].ident {
        format!(
            "Terminal type selector `{name}`. Matches the view's runtime type name; custom names are supported."
        )
    } else if s.kind == Kind::Media
        && matches!(
            name,
            "min-width" | "max-width" | "min-height" | "max-height"
        )
    {
        format!(
            "`{name}` compares the root terminal viewport size in cells. Combine conditions with `and`."
        )
    } else if token == s.head.start + 1 && syntax.at(s.head.start) == "@" {
        match name {
            "use" => "Imports a StyleModule helper function; use `as` to choose a local namespace.",
            "mixin" => "Defines a reusable group of declarations and mixin includes.",
            "include" => "Includes a local or imported mixin; terminate the include with `;`.",
            "media" => "Applies rules or declarations conditionally to the terminal viewport size.",
            _ => return None,
        }
        .into()
    } else {
        return None;
    };
    Some(json!({"contents":{"kind":"markdown","value":detail},"range":range(text,word)}))
}

/// Extends Rust helper renames to namespace references from implicit imports.
///
/// # Arguments
///
/// * `documents` — Workspace source documents, including unsaved overlays.
/// * `edits` — Workspace edits augmented in place.
/// * `new_name` — New name selected for the Rust helper.
///
/// # Returns
///
/// Additional reference locations whose namespaces follow the renamed helper.
pub(crate) fn augment_aliases(
    documents: &std::collections::HashMap<String, String>,
    edits: &mut std::collections::HashMap<String, Vec<Value>>,
    new_name: &str,
) -> Vec<Value> {
    let mut locations = Vec::new();
    for (uri, text) in documents {
        for import in imports(text).into_iter().filter(|i| !i.explicit) {
            if !edits.get(uri).is_some_and(|edits| {
                edits
                    .iter()
                    .any(|e| e["range"] == range(text, import.path.clone()))
            }) {
                continue;
            }
            for symbol in symbols(text).into_iter().filter(|s| {
                s.kind == "module"
                    && !s.declaration
                    && s.scope == import.scope
                    && s.name == import.alias
            }) {
                let span = range(text, symbol.span);
                let edit = json!({"range":span,"newText":new_name});
                let target = edits.entry(uri.clone()).or_default();
                if !target.contains(&edit) {
                    target.push(edit);
                }
                locations.push(json!({"uri":uri,"range":span}));
            }
        }
    }
    locations
}

#[cfg(test)]
mod tests {
    //! Recovery and source-edit regression coverage.
    use super::*;

    /// Verifies properties remain available inside incomplete nested rules.
    ///
    /// # Example Under Test
    ///
    /// `stylesheet! { .panel { @media (max-width: 80) { fg`.
    ///
    /// # Assertions
    ///
    /// - The property completion inserts a value slot and terminator.
    #[test]
    fn incomplete_nested_rule() {
        let text = "stylesheet! { .panel { @media (max-width: 80) { fg";
        let items = complete(text, text.len(), true).unwrap();
        assert_eq!(items[0]["textEdit"]["newText"], "fg: $0;");
    }

    /// Verifies expression completion remains delegated to rust-analyzer.
    ///
    /// # Example Under Test
    ///
    /// `stylesheet! { Text { fg: Color::`.
    ///
    /// # Assertions
    ///
    /// - No stylesheet items replace the Rust completion result.
    #[test]
    fn rust_values_are_delegated() {
        let text = "stylesheet! { Text { fg: Color::";
        assert!(complete(text, text.len(), true).is_none());
    }

    /// Verifies comments, strings, and unrelated macros are excluded.
    ///
    /// # Example Under Test
    ///
    /// A string, comment, and ordinary Rust function.
    ///
    /// # Assertions
    ///
    /// - None of these positions receive stylesheet completions.
    #[test]
    fn excludes_non_stylesheet_positions() {
        for text in [
            "fn main() { fg",
            "stylesheet! { /* fg  */",
            "stylesheet! { // fg",
            "stylesheet! { Text { fg: \"unfinished",
            "stylesheet! { Text { fg: \"abc\" }",
        ] {
            let cursor = if text.contains("/*") {
                text.find("fg").unwrap() + 2
            } else if text.contains("abc") {
                text.find("abc").unwrap() + 2
            } else {
                text.len()
            };
            assert!(complete(text, cursor, true).is_none(), "{text}");
        }
    }

    /// Verifies local variable and mixin names complete without snippet syntax.
    ///
    /// # Example Under Test
    ///
    /// A local `$accent` variable and `chrome` mixin.
    ///
    /// # Assertions
    ///
    /// - Only the referenced namespace is suggested.
    #[test]
    fn local_names_and_plain_text() {
        for (text, name) in [
            (
                "stylesheet! { $accent: Color::Red; Text { fg: $ac",
                "accent",
            ),
            (
                "stylesheet! { @mixin chrome { fg: Color::Red; } Text { @include ch",
                "chrome",
            ),
        ] {
            let items = complete(text, text.len(), false).unwrap();
            assert_eq!(items.len(), 1);
            assert_eq!(items[0]["label"], name);
            assert_eq!(items[0]["insertTextFormat"], 1);
        }
    }
    /// Verifies surrounding Rust scopes do not change root or value completion contexts.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// fn main() { stylesheet! {  } }
    /// fn main() { stylesheet! { Text { fg: Color::  } } }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Root contexts exclude properties and whitespace after a Rust path remains upstream.
    #[test]
    fn contexts_ignore_outer_scopes_and_trailing_whitespace() {
        let root = "fn main() { stylesheet! {  } }";
        let items = complete(root, root.find("  }").unwrap() + 1, true).unwrap();
        assert!(!items.iter().any(|i| i["label"] == "fg"));
        let value = "fn main() { stylesheet! { Text { fg: Color::  } } }";
        assert!(complete(value, value.find("  }").unwrap() + 1, true).is_none());
    }
}
