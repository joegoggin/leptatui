//! Semantic stylesheet import resolution using rust-analyzer definitions.

use super::{
    Symbol, imports, range, symbols,
    syntax::{Kind, Syntax},
};
use crate::{Proxy, source};
use serde_json::{Value, json};
use std::{collections::HashMap, io};

/// A resolved static module export target.
#[derive(Clone)]
struct Link {
    /// Importing source document.
    uri: String,
    /// Importing stylesheet invocation.
    scope: usize,
    /// Imported namespace.
    alias: String,
    /// Source document containing the helper function.
    target_uri: String,
    /// Exporting stylesheet invocation, if its exports are statically known.
    target_scope: Option<usize>,
}

/// Revision-scoped resolved imports, including source overlays read for dependencies.
#[derive(Clone)]
pub(crate) struct Snapshot {
    /// Revision against which definitions were resolved.
    revision: u64,
    /// Source documents including imported helpers.
    documents: HashMap<String, String>,
    /// Resolved import targets.
    links: Vec<Link>,
}

/// Returns a definition URI and selection range from either LSP location representation.
///
/// # Arguments
///
/// * `value` — Definition result returned by rust-analyzer.
///
/// # Returns
///
/// An optional definition URI and selection range.
fn definition(value: &Value) -> Option<(String, Value)> {
    if let Some(items) = value.as_array() {
        return items.iter().find_map(definition);
    }
    Some((
        value["uri"]
            .as_str()
            .or_else(|| value["targetUri"].as_str())?
            .into(),
        value
            .get("range")
            .or_else(|| value.get("targetSelectionRange"))?
            .clone(),
    ))
}

/// Identifies a statically returned module in the resolved helper function.
///
/// # Arguments
///
/// * `text` — Source document with unsaved edits applied.
/// * `position` — Definition selection position in UTF-16 coordinates.
///
/// # Returns
///
/// An optional invocation index for a directly returned stylesheet module.
fn export_scope(text: &str, position: &Value) -> Option<usize> {
    let offset = source::offset(text, position)?;
    let syntax = Syntax::new(text);
    let index = syntax
        .tokens
        .iter()
        .position(|t| t.span.start <= offset && offset < t.span.end)?;
    if index == 0 || syntax.at(index - 1) != "fn" {
        return None;
    }
    let open = (index + 1..syntax.tokens.len()).find(|&i| matches!(syntax.at(i), "{" | ";"))?;
    if syntax.at(open) != "{" {
        return None;
    }
    let end = syntax.end(open, syntax.tokens.len());
    let candidates: Vec<_> = syntax
        .macros
        .iter()
        .filter(|m| !m.view && m.open > open && m.end < end)
        .collect();
    if candidates.len() != 1 {
        return None;
    }
    let m = candidates[0];
    // Only a direct return has a complete, statically known export set.
    let prefix = &syntax.tokens[open + 1..m.open - 1];
    let direct = prefix.iter().all(|t| t.ident || t.text == ":");
    let tail = &syntax.tokens[m.end + 1..end];
    let returned = prefix.first().is_some_and(|t| t.text == "return");
    if !direct || !(tail.is_empty() || returned && tail.len() == 1 && tail[0].text == ";") {
        return None;
    }
    let tree = syntax.parse(m);
    if tree
        .statements
        .iter()
        .any(|s| matches!(s.kind, Kind::Rule | Kind::Media))
    {
        return None;
    }
    Some(m.open)
}

impl Proxy {
    /// Reports missing members only when a helper's exports are statically known.
    ///
    /// # Arguments
    ///
    /// * `uri` — Requested source document URI.
    ///
    /// # Returns
    ///
    /// A list of missing-member diagnostics for statically known exports.
    pub(crate) fn import_diagnostics(&self, uri: &str) -> Vec<Value> {
        let Some(snapshot) = &self.stylesheet_imports else {
            return Vec::new();
        };
        if snapshot.revision != self.revision {
            return Vec::new();
        }
        let Some(text) = self.documents.get(uri) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for symbol in symbols(text).into_iter().filter(|s| s.alias.is_some()) {
            if let Some(link) = snapshot.links.iter().find(|l| {
                l.uri == uri && l.scope == symbol.scope && Some(&l.alias) == symbol.alias.as_ref()
            }) && let Some(scope) = link.target_scope
                && let Some(target) = snapshot.documents.get(&link.target_uri)
                && !symbols(target).iter().any(|s| {
                    s.scope == scope
                        && s.declaration
                        && s.name == symbol.name
                        && s.kind == symbol.kind
                })
            {
                result.push(super::diagnostics::diagnostic(
                    text,
                    symbol.span,
                    format!(
                        "unknown imported stylesheet {} `{}`",
                        symbol.kind, symbol.name
                    ),
                ));
            }
        }
        result
    }

    /// Resolves static imports after edits so missing-member diagnostics do not require a hover.
    ///
    /// # Returns
    ///
    /// An empty result after refreshing diagnostics for stable source overlays.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the upstream or client transport fails.
    pub(crate) fn refresh_style_imports(&mut self) -> io::Result<()> {
        if !self
            .documents
            .values()
            .any(|text| !imports(text).is_empty())
        {
            return Ok(());
        }
        self.cancelled = false;
        // Requests queued during recovery take priority over another diagnostic refresh.
        loop {
            let revision = self.revision;
            self.style_imports()?;
            if self.revision == revision || !self.pending.is_empty() {
                break;
            }
        }
        Ok(())
    }

    /// Reads workspace sources and overlays unsaved documents.
    ///
    /// # Returns
    ///
    /// A map of workspace Rust sources with unsaved overlays.
    pub(crate) fn workspace(&self) -> HashMap<String, String> {
        let mut documents = self.documents.clone();
        for root in &self.roots {
            source::documents(root, &mut documents);
        }
        documents
    }

    /// Resolves authored imports through the Rust language server, preserving aliases.
    ///
    /// # Returns
    ///
    /// A revision-scoped import graph with its source documents.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the upstream or client transport fails.
    fn style_imports(&mut self) -> io::Result<Snapshot> {
        if let Some(snapshot) = &self.stylesheet_imports
            && snapshot.revision == self.revision
        {
            return Ok(snapshot.clone());
        }
        let revision = self.revision;
        let mut documents = self.workspace();
        let mut links = Vec::new();
        let pending: Vec<_> = documents
            .iter()
            .flat_map(|(uri, text)| {
                imports(text)
                    .into_iter()
                    .map(move |import| (uri.clone(), text.clone(), import))
            })
            .collect();
        for (uri, text, import) in pending {
            let response=self.request("textDocument/definition",json!({"textDocument":{"uri":uri},"position":range(&text,import.path.clone())["start"]}))?;
            if self.cancelled || self.revision != revision {
                break;
            }
            let Some((target_uri, selection)) = definition(&response["result"]) else {
                continue;
            };
            if !documents.contains_key(&target_uri)
                && let Ok(url) = url::Url::parse(&target_uri)
                && let Ok(path) = url.to_file_path()
                && let Ok(text) = std::fs::read_to_string(path)
            {
                documents.insert(target_uri.clone(), text);
            }
            let target_scope = documents
                .get(&target_uri)
                .and_then(|text| export_scope(text, &selection["start"]));
            links.push(Link {
                uri,
                scope: import.scope,
                alias: import.alias,
                target_uri,
                target_scope,
            });
        }
        let snapshot = Snapshot {
            revision,
            documents,
            links,
        };
        if self.revision == revision && !self.cancelled {
            self.stylesheet_imports = Some(snapshot.clone());
            for uri in self.documents.keys().cloned().collect::<Vec<_>>() {
                self.publish_style_diagnostics(&uri)?;
            }
        }
        Ok(snapshot)
    }

    /// Rejects namespace collisions when renaming local or imported stylesheet symbols.
    ///
    /// # Arguments
    ///
    /// * `uri` — Requested source document URI.
    /// * `cursor` — Request byte offset at a UTF-8 boundary.
    /// * `name` — Proposed replacement symbol or selector name.
    ///
    /// # Returns
    ///
    /// An optional namespace-collision error.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the upstream or client transport fails.
    pub(crate) fn style_rename_error(
        &mut self,
        uri: &str,
        cursor: usize,
        name: &str,
    ) -> io::Result<Option<String>> {
        let Some(text) = self.documents.get(uri).cloned() else {
            return Ok(None);
        };
        let local = symbols(&text);
        let Some(selected) = local.iter().find(|s| s.contains(&text, cursor)) else {
            return Ok(None);
        };
        let snapshot = self.style_imports()?;
        let target = if let Some(alias) = &selected.alias {
            snapshot
                .links
                .iter()
                .find(|l| l.uri == uri && l.scope == selected.scope && &l.alias == alias)
                .and_then(|l| l.target_scope.map(|scope| (l.target_uri.as_str(), scope)))
        } else {
            Some((uri, selected.scope))
        };
        if let Some((doc, scope)) = target
            && let Some(text) = snapshot.documents.get(doc)
            && name != selected.name
            && symbols(text).iter().any(|s| {
                s.scope == scope && s.kind == selected.kind && s.declaration && s.name == name
            })
        {
            return Ok(Some(format!(
                "stylesheet {} `{name}` already exists",
                selected.kind
            )));
        }
        Ok(None)
    }

    /// Completes imported variables and mixins using statically discoverable exports.
    ///
    /// # Arguments
    ///
    /// * `uri` — Requested source document URI.
    /// * `cursor` — Request byte offset at a UTF-8 boundary.
    ///
    /// # Returns
    ///
    /// An optional list of imported-member completions.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the upstream or client transport fails.
    pub(crate) fn imported_completions(
        &mut self,
        uri: &str,
        cursor: usize,
    ) -> io::Result<Option<Vec<Value>>> {
        let Some(text) = self.documents.get(uri).cloned() else {
            return Ok(None);
        };
        let syntax = Syntax::new(&text);
        let Some(m) = syntax.stylesheet(cursor) else {
            return Ok(None);
        };
        if syntax.excluded(cursor) {
            return Ok(None);
        }
        let tree = syntax.parse(m);
        let (_, statement) = tree.context(cursor);
        let Some(statement) = statement else {
            return Ok(None);
        };
        if !matches!(statement.kind, Kind::Property | Kind::Include) {
            return Ok(None);
        }
        let word = syntax.word(cursor);
        let before: Vec<_> = syntax.tokens[statement.head.clone()]
            .iter()
            .filter(|t| t.span.end <= word.start)
            .collect();
        let Some(dot) = before.iter().rposition(|t| t.text == ".") else {
            return Ok(None);
        };
        if dot == 0
            || before.len() > dot + 2 && !(before.len() == dot + 3 && before[dot + 1].text == "$")
        {
            return Ok(None);
        }
        let alias = before[dot - 1].text.to_owned();
        let variable = statement.kind == Kind::Property;
        if variable && before.last().is_none_or(|t| !matches!(t.text, "." | "$")) {
            return Ok(None);
        }
        if !imports(&text)
            .iter()
            .any(|i| i.scope == m.open && i.alias == alias)
        {
            return Ok(None);
        }
        let snapshot = self.style_imports()?;
        let link = snapshot
            .links
            .iter()
            .find(|l| l.uri == uri && l.scope == m.open && l.alias == alias);
        let mut items = Vec::new();
        if let Some(link) = link
            && let Some(scope) = link.target_scope
            && let Some(target) = snapshot.documents.get(&link.target_uri)
        {
            for symbol in symbols(target).into_iter().filter(|s| {
                s.scope == scope
                    && s.declaration
                    && s.kind == if variable { "variable" } else { "mixin" }
            }) {
                if !symbol.name.starts_with(&text[word.start..cursor]) {
                    continue;
                }
                let insert = if variable && before.last().is_some_and(|t| t.text == ".") {
                    format!("${}", symbol.name)
                } else {
                    symbol.name.clone()
                };
                items.push(json!({"label":symbol.name,"kind":6,"detail":format!("Imported {} from {}",symbol.kind,alias),"textEdit":{"range":range(&text,word.clone()),"newText":insert},"insertTextFormat":1,"data":{"leptatuiStylesheet":true}}));
            }
        }
        Ok(Some(items))
    }

    /// Answers navigation and refactoring requests across statically resolved modules.
    ///
    /// # Arguments
    ///
    /// * `uri` — Requested source document URI.
    /// * `cursor` — Request byte offset at a UTF-8 boundary.
    /// * `method` — Requested LSP method.
    /// * `params` — Original request parameters.
    ///
    /// # Returns
    ///
    /// An optional semantic result for a local or imported stylesheet symbol.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the upstream or client transport fails.
    pub(crate) fn imported_semantic(
        &mut self,
        uri: &str,
        cursor: usize,
        method: &str,
        params: &Value,
    ) -> io::Result<Option<Value>> {
        let Some(text) = self.documents.get(uri).cloned() else {
            return Ok(None);
        };
        let local = symbols(&text);
        let Some(selected) = local.iter().find(|s| s.contains(&text, cursor)) else {
            return Ok(None);
        };
        if selected.kind == "module" {
            return Ok(None);
        }
        let snapshot = self.style_imports()?;
        let target = if let Some(alias) = &selected.alias {
            snapshot
                .links
                .iter()
                .find(|l| l.uri == uri && l.scope == selected.scope && &l.alias == alias)
                .and_then(|l| l.target_scope.map(|scope| (l.target_uri.clone(), scope)))
        } else {
            Some((uri.to_owned(), selected.scope))
        };
        let Some((target_uri, target_scope)) = target else {
            return Ok(None);
        };
        let mut occurrences: Vec<(String, Symbol)> = Vec::new();
        for (doc, text) in &snapshot.documents {
            for symbol in symbols(text)
                .into_iter()
                .filter(|s| s.name == selected.name && s.kind == selected.kind)
            {
                let matches = if let Some(alias) = &symbol.alias {
                    snapshot.links.iter().any(|l| {
                        &l.uri == doc
                            && l.scope == symbol.scope
                            && &l.alias == alias
                            && l.target_uri == target_uri
                            && l.target_scope == Some(target_scope)
                    })
                } else {
                    doc == &target_uri && symbol.scope == target_scope
                };
                if matches {
                    occurrences.push((doc.clone(), symbol));
                }
            }
        }
        let declaration = occurrences.iter().find(|(_, s)| s.declaration);
        let location = |(uri, s): &(String, Symbol)| json!({"uri":uri,"range":range(&snapshot.documents[uri],s.span.clone())});
        let result=match method {
            "textDocument/definition"=>Some(declaration.map(location).unwrap_or(Value::Null)),
            "textDocument/hover"=>declaration.map(|_|json!({"contents":{"kind":"markdown","value":format!("Stylesheet {} `{}`",selected.kind,selected.name)},"range":range(&text,selected.span.clone())})),
            "textDocument/references"=>Some(Value::Array(occurrences.iter().filter(|(_,s)|!s.declaration || params["context"]["includeDeclaration"]==true).map(location).collect())),
            "textDocument/prepareRename"=>declaration.map(|_|json!({"range":range(&text,selected.span.clone()),"placeholder":selected.name})),
            "textDocument/rename"=>{
                let name=params["newName"].as_str().unwrap_or("");
                if declaration.is_none() || syn::parse_str::<syn::Ident>(name).is_err() {None}
                else {
                    let mut changes:HashMap<String,Vec<Value>>=HashMap::new();
                    for (uri,s) in &occurrences {changes.entry(uri.clone()).or_default().push(json!({"range":range(&snapshot.documents[uri],s.span.clone()),"newText":name}));}
                    Some(json!({"changes":changes}))
                }
            }
            _=>None,
        };
        Ok(result)
    }
}
