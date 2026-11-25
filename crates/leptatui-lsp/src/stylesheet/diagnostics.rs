//! Stylesheet diagnostics merged with rust-analyzer's compiler and type diagnostics.

use super::{
    properties, range, symbols,
    syntax::{Block, Kind, Syntax},
};
use crate::{Proxy, protocol};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    io,
    ops::Range,
};

/// Constructs one source-ranged stylesheet diagnostic.
///
/// # Arguments
///
/// * `text` — Source document with unsaved edits applied.
/// * `span` — Source byte range.
/// * `message` — Human-readable diagnostic description.
///
/// # Returns
///
/// A source-ranged stylesheet error diagnostic.
pub(super) fn diagnostic(text: &str, span: Range<usize>, message: impl Into<String>) -> Value {
    json!({"range":range(text,span),"severity":1,"source":"leptatui","message":message.into()})
}

/// Checks declarations and structural restrictions without parsing Rust value expressions.
///
/// # Arguments
///
/// * `syntax` — Tokenized source and recoverable delimiter pairs.
/// * `block` — Stylesheet block to inspect.
/// * `parent` — Containing selector or block context.
/// * `class` — Whether the current terminal selector is a class.
/// * `output` — Destination for collected results.
fn block_diagnostics(
    syntax: &Syntax<'_>,
    block: &Block,
    parent: Option<Kind>,
    class: bool,
    output: &mut Vec<Value>,
) {
    for s in &block.statements {
        let start = s.head.start;
        let first = syntax.at(start);
        let span = syntax.tokens[start].span.clone();
        let mut error = None;
        match s.kind {
            Kind::Property=>{
                if !block.declarations {error=Some("style declarations require a selector or mixin block".to_owned());}
                else if !properties::PROPERTIES.iter().any(|p|p.0==first) {error=Some(format!("unsupported stylesheet declaration `{first}`"));}
            }
            Kind::Variable|Kind::Mixin|Kind::Use if parent.is_some()=>error=Some("variables, mixins, and imports must be declared at the stylesheet root".into()),
            Kind::Include if !block.declarations=>error=Some("@include requires a selector or mixin block".into()),
            Kind::Media if parent==Some(Kind::Mixin)=>error=Some("@media is not supported inside a mixin".into()),
            Kind::Media if parent==Some(Kind::Media)=>error=Some("nested @media blocks cannot directly contain another @media block; combine conditions with `and`".into()),
            Kind::Rule=>{
                if !block.rules {error=Some("nested selectors are not supported inside a mixin".into());}
                if first=="&" && parent.is_none() {error=Some("parent selectors require a containing selector".into());}
                else if first=="&" && syntax.at(start+1)!=":" && !class {error=Some("parent class suffix requires a class selector".into());}
                for i in s.head.clone() {
                    if syntax.at(i)==":" && !matches!(syntax.at(i+1),"focus"|"active"|"insert"|"visual"|"visited"|"") {
                        output.push(diagnostic(syntax.text,syntax.tokens[i+1].span.clone(),"unsupported stylesheet pseudo-selector; expected :focus, :active, :insert, :visual, or :visited"));
                    }
                }
            }
            _=>{}
        }
        if let Some(message) = error {
            output.push(diagnostic(syntax.text, span.clone(), message));
        }
        if matches!(
            s.kind,
            Kind::Property | Kind::Variable | Kind::Include | Kind::Use
        ) && !s.terminated
        {
            output.push(diagnostic(
                syntax.text,
                s.span.end..s.span.end,
                if s.kind == Kind::Property {
                    "stylesheet! declarations must end with `;`"
                } else {
                    "expected `;`"
                },
            ));
        }
        if let Some(body) = &s.body {
            let class = if s.kind == Kind::Rule {
                first == "." || first == "&" && class
            } else {
                class
            };
            block_diagnostics(syntax, body, Some(s.kind), class, output);
        }
    }
}

/// Returns syntax and local-symbol diagnostics for an open stylesheet document.
///
/// # Arguments
///
/// * `text` — Source document with unsaved edits applied.
///
/// # Returns
///
/// A list of stylesheet syntax and local-symbol diagnostics.
pub(crate) fn diagnostics(text: &str) -> Vec<Value> {
    let syntax = Syntax::new(text);
    let mut output = Vec::new();
    let symbols = symbols(text);
    for m in syntax.macros.iter().filter(|m| !m.view) {
        let tree = syntax.parse(m);
        block_diagnostics(&syntax, &tree, None, false, &mut output);
        let local: Vec<_> = symbols
            .iter()
            .filter(|s| s.scope == m.open && s.alias.is_none())
            .collect();
        let mut definitions = HashSet::new();
        for s in local.iter().filter(|s| s.declaration) {
            if !definitions.insert((s.kind, s.name.as_str())) {
                output.push(diagnostic(
                    text,
                    s.span.clone(),
                    format!("duplicate stylesheet {} `{}`", s.kind, s.name),
                ));
            }
        }
        for s in local.iter().filter(|s| !s.declaration) {
            if !definitions.contains(&(s.kind, s.name.as_str())) {
                output.push(diagnostic(
                    text,
                    s.span.clone(),
                    format!("unknown stylesheet {} `{}`", s.kind, s.name),
                ));
            }
        }
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for s in tree.statements.iter().filter(|s| s.kind == Kind::Mixin) {
            if let Some(body) = &s.body {
                let name = syntax.at(s.head.start + 2).to_owned();
                let deps = local
                    .iter()
                    .filter(|symbol| {
                        symbol.kind == "mixin"
                            && !symbol.declaration
                            && body.span.contains(&symbol.span.start)
                    })
                    .map(|s| s.name.clone())
                    .collect();
                graph.insert(name, deps);
            }
        }
        for s in local.iter().filter(|s| s.kind == "mixin" && s.declaration) {
            if cycle(&s.name, &s.name, &graph, &mut HashSet::new()) {
                output.push(diagnostic(
                    text,
                    s.span.clone(),
                    format!("cyclic stylesheet mixin `{}`", s.name),
                ));
            }
        }
    }
    output
}

/// Checks whether a mixin dependency path returns to its starting declaration.
///
/// # Arguments
///
/// * `start` — Original mixin whose dependency cycle is being checked.
/// * `current` — Mixin currently being visited.
/// * `graph` — Mixin include dependencies.
/// * `visited` — Mixin names already visited along this search.
///
/// # Returns
///
/// A boolean indicating whether the dependency path returns to its starting mixin.
fn cycle(
    start: &str,
    current: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
) -> bool {
    if !visited.insert(current.into()) {
        return false;
    }
    graph.get(current).is_some_and(|deps| {
        deps.iter()
            .any(|next| next == start || cycle(start, next, graph, visited))
    })
}

impl Proxy {
    /// Adds stylesheet capabilities and merges local diagnostics into upstream messages.
    ///
    /// # Arguments
    ///
    /// * `message` — Protocol message to inspect or forward.
    ///
    /// # Returns
    ///
    /// An empty result after forwarding the augmented server message.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the upstream or client transport fails.
    pub(crate) fn forward_server(&mut self, mut message: Value) -> io::Result<()> {
        if let Some(capabilities) = message.pointer_mut("/result/capabilities") {
            if capabilities["completionProvider"].is_object() {
                if !capabilities["completionProvider"]["triggerCharacters"].is_array() {
                    capabilities["completionProvider"]["triggerCharacters"] = json!([]);
                }
                let triggers = capabilities["completionProvider"]["triggerCharacters"]
                    .as_array_mut()
                    .unwrap();
                for trigger in ["$", "@", "&", "#", "!", ".", ":"] {
                    if !triggers.iter().any(|v| v == trigger) {
                        triggers.push(json!(trigger));
                    }
                }
            }
            capabilities["hoverProvider"] = json!(true);
            capabilities["definitionProvider"] = json!(true);
            capabilities["referencesProvider"] = json!(true);
            capabilities["renameProvider"] = json!({"prepareProvider":true});
        }
        if message["method"] == "textDocument/publishDiagnostics"
            && let Some(uri) = message["params"]["uri"].as_str().map(str::to_owned)
        {
            self.stylesheet_upstream_diagnostics
                .insert(uri.clone(), message.clone());
            self.merge_style_diagnostics(&uri, &mut message);
        }
        protocol::write(&mut self.client, &message)
    }

    /// Merges source-ranged stylesheet diagnostics without repeating compiler errors.
    ///
    /// # Arguments
    ///
    /// * `uri` — Requested source document URI.
    /// * `message` — Protocol message to inspect or forward.
    fn merge_style_diagnostics(&self, uri: &str, message: &mut Value) {
        let Some(text) = self.documents.get(uri) else {
            return;
        };
        let Some(items) = message["params"]["diagnostics"].as_array_mut() else {
            return;
        };
        let mut additions = diagnostics(text);
        additions.extend(self.import_diagnostics(uri));
        for diagnostic in additions {
            let message = diagnostic["message"].as_str().unwrap_or("");
            if !items.iter().any(|d| {
                d["severity"] == 1
                    && (d["range"] == diagnostic["range"]
                        || d["message"].as_str().is_some_and(|m| m.contains(message)))
            }) {
                items.push(diagnostic);
            }
        }
    }

    /// Publishes updated diagnostics after an open document changes or imports resolve.
    ///
    /// # Arguments
    ///
    /// * `uri` — Requested source document URI.
    ///
    /// # Returns
    ///
    /// An empty result after publishing the merged diagnostic notification.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the upstream or client transport fails.
    pub(crate) fn publish_style_diagnostics(&mut self, uri: &str) -> io::Result<()> {
        let mut message=self.stylesheet_upstream_diagnostics.get(uri).cloned().unwrap_or_else(||json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"diagnostics":[]}}));
        // Never replay a version from an earlier upstream document snapshot.
        if let Some(params) = message["params"].as_object_mut() {
            params.remove("version");
        }
        self.merge_style_diagnostics(uri, &mut message);
        protocol::write(&mut self.client, &message)
    }
}

#[cfg(test)]
mod tests {
    //! Diagnostics without invoking the upstream server.
    use super::*;

    /// Verifies Rust fields are not mistaken for stylesheet module references.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// stylesheet! { Text { fg: person.color; } }
    /// ```
    ///
    /// # Assertions
    ///
    /// - The stylesheet contributes no syntax or namespace diagnostics.
    #[test]
    fn rust_fields_are_not_modules() {
        assert!(diagnostics("stylesheet! { Text { fg: person.color; } }").is_empty());
    }

    /// Verifies duplicate names, unknown references, and recursive mixins are diagnosed.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// stylesheet! { $x: 1; $x: 2; @mixin a { @include b; } @mixin b { @include a; } Text { fg: $missing; } }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Duplicate, unknown-variable, and mixin-cycle diagnostics are produced.
    #[test]
    fn namespace_and_cycle_errors() {
        let errors = diagnostics(
            "stylesheet! { $x: 1; $x: 2; @mixin a { @include b; } @mixin b { @include a; } Text { fg: $missing; } }",
        );
        let messages = errors
            .iter()
            .map(|e| e["message"].as_str().unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(messages.contains("duplicate"));
        assert!(messages.contains("unknown"));
        assert!(messages.contains("cyclic"));
    }
}
