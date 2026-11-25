//! Request routing and completion lifecycle for stylesheet editor features.

use crate::{Proxy, protocol, selectors, source};
use serde_json::{Value, json};
use std::io;

impl Proxy {
    /// Handles stylesheet semantic requests, preserving revision and cancellation checks.
    ///
    /// # Arguments
    ///
    /// * `message` — Protocol message to inspect or forward.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the request was answered locally.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the upstream or client transport fails.
    pub(crate) fn style_semantic(&mut self, message: &Value) -> io::Result<bool> {
        let method = message["method"].as_str().unwrap_or("");
        if !matches!(
            method,
            "textDocument/hover"
                | "textDocument/definition"
                | "textDocument/references"
                | "textDocument/rename"
                | "textDocument/prepareRename"
        ) {
            return Ok(false);
        }
        let Some(uri) = message["params"]["textDocument"]["uri"].as_str() else {
            return Ok(false);
        };
        let Some(text) = self.documents.get(uri).cloned() else {
            return Ok(false);
        };
        let Some(cursor) = source::offset(&text, &message["params"]["position"]) else {
            return Ok(false);
        };
        if super::symbols(&text).iter().any(|s| {
            s.kind == "module"
                && s.declaration
                && s.span.start <= cursor
                && cursor <= s.span.end
                && super::imports(&text)
                    .iter()
                    .any(|i| i.scope == s.scope && !i.explicit && i.alias == s.name)
        }) {
            return Ok(false);
        }
        let local = std::collections::HashMap::from([(uri.to_owned(), text.clone())]);
        let is_selector = selectors::semantic(
            &local,
            uri,
            cursor,
            "textDocument/hover",
            &message["params"],
        )
        .is_some();
        let is_symbol = super::symbols(&text)
            .iter()
            .any(|s| s.contains(&text, cursor));
        if !is_selector
            && !is_symbol
            && method != "textDocument/hover"
            && method != "textDocument/definition"
        {
            return Ok(false);
        }
        let revision = self.revision;
        self.active = Some(message["id"].clone());
        self.cancelled = false;
        let result = (|| {
            if method == "textDocument/rename" {
                let name = message["params"]["newName"].as_str().unwrap_or("");
                let error = if is_selector {
                    selectors::rename_error(&self.workspace(), uri, cursor, name)
                } else if is_symbol && syn::parse_str::<syn::Ident>(name).is_err() {
                    Some("stylesheet symbol names must be Rust identifiers".into())
                } else if is_symbol {
                    self.style_rename_error(uri, cursor, name)?
                } else {
                    None
                };
                if let Some(error) = error {
                    protocol::write(
                        &mut self.client,
                        &json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32602,"message":error}}),
                    )?;
                    return Ok(true);
                }
            }
            let result = if is_selector {
                selectors::semantic(&self.workspace(), uri, cursor, method, &message["params"])
            } else if is_symbol {
                self.imported_semantic(uri, cursor, method, &message["params"])?
                    .or_else(|| super::semantic(&text, uri, cursor, method, &message["params"]))
            } else {
                super::semantic(&text, uri, cursor, method, &message["params"])
            };
            if let Some(result) = result {
                let response = if self.cancelled {
                    json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32800,"message":"request cancelled"}})
                } else if self.revision != revision {
                    json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32801,"message":"document changed during stylesheet request; retry"}})
                } else {
                    json!({"jsonrpc":"2.0","id":message["id"],"result":result})
                };
                protocol::write(&mut self.client, &response)?;
                Ok(true)
            } else {
                Ok(false)
            }
        })();
        self.active = None;
        result
    }

    /// Completes authored stylesheet syntax, imported members, or literal selector names.
    ///
    /// # Arguments
    ///
    /// * `params` — Original request parameters.
    ///
    /// # Returns
    ///
    /// An optional list of stylesheet or literal-selector completions.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the upstream or client transport fails.
    pub(crate) fn style_completions(&mut self, params: &Value) -> io::Result<Option<Vec<Value>>> {
        let Some(uri) = params["textDocument"]["uri"].as_str() else {
            return Ok(None);
        };
        let Some(text) = self.documents.get(uri).cloned() else {
            return Ok(None);
        };
        let Some(cursor) = source::offset(&text, &params["position"]) else {
            return Ok(None);
        };
        if let Some(items) = self.imported_completions(uri, cursor)? {
            return Ok(Some(items));
        }
        let local = std::collections::HashMap::from([(uri.to_owned(), text.clone())]);
        if selectors::complete(&local, uri, cursor).is_some() {
            return Ok(selectors::complete(&self.workspace(), uri, cursor));
        }
        let mut items = super::complete(&text, cursor, self.snippets);
        if let Some(items) = &mut items {
            if let Some(command) = &self.completion_command {
                for item in items.iter_mut().filter(|item| {
                    item["kind"] == 10
                        && super::properties::PROPERTIES
                            .iter()
                            .any(|p| item["label"] == p.0)
                        && item["textEdit"]["newText"]
                            .as_str()
                            .is_some_and(|text| text.contains("$0"))
                }) {
                    item["command"] = json!({"title":"Suggest property values", "command":command});
                }
            }
            let extras = super::type_completions(&self.workspace(), uri, cursor, self.snippets);
            for extra in extras {
                if !items.iter().any(|item| item["label"] == extra["label"]) {
                    items.push(extra);
                }
            }
        }
        Ok(items)
    }
}
