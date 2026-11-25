//! Completion snippets using rust-analyzer's resolved component documentation.
//!
//! Component macros publish the child-content contract in their ordinary rustdoc.
//! Keeping it on the resolved symbol preserves aliases and avoids a second Rust
//! name resolver in the proxy.
//!
//! # Modules
//!
//! - [`context`] — Recovering markup positions and existing tag structure.

mod context;

use crate::{Proxy, protocol, source};
use context::Context;
use serde_json::{Value, json};
use std::{io, ops::Range};

/// Source context retained without changing rust-analyzer's opaque item data.
#[derive(Clone)]
pub(crate) struct Snapshot {
    /// Document from which the completion was requested.
    uri: String,
    /// Document revision against which the edit was computed.
    revision: u64,
    /// Opening tag or bare markup prefix.
    context: Context,
}

impl Proxy {
    /// Decorates tag and macro completion without changing rust-analyzer identity.
    ///
    /// # Arguments
    ///
    /// * `message` — Original completion or completion-item resolve request.
    ///
    /// # Returns
    ///
    /// An empty result after forwarding the completed response.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if either transport fails.
    pub(crate) fn complete(&mut self, message: Value) -> io::Result<()> {
        self.active = Some(message["id"].clone());
        self.cancelled = false;
        let result = self.complete_inner(&message);
        self.active = None;
        result
    }

    /// Resolves component metadata and applies edits against one document revision.
    ///
    /// # Arguments
    ///
    /// * `message` — Original completion request.
    ///
    /// # Returns
    ///
    /// An empty result after sending a response with the original request ID.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if a transport fails.
    fn complete_inner(&mut self, message: &Value) -> io::Result<()> {
        if message["method"] == "completionItem/resolve"
            && message["params"]["data"]["leptatuiStylesheet"] == true
        {
            let response =
                json!({"jsonrpc":"2.0","id":message["id"],"result":message["params"]});
            return protocol::write(&mut self.client, &response);
        }
        let revision = self.revision;
        if message["method"] == "textDocument/completion"
            && let Some(mut items) = self.style_completions(&message["params"])?
        {
            for item in &mut items {
                item["data"]["revision"] = json!(revision);
            }
            let response = if self.cancelled {
                json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32800,"message":"request cancelled"}})
            } else if self.revision != revision {
                json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32801,"message":"document changed during completion; retry"}})
            } else {
                json!({"jsonrpc":"2.0","id":message["id"],"result":{"isIncomplete":false,"items":items}})
            };
            return protocol::write(&mut self.client, &response);
        }

        let method = message["method"].as_str().unwrap_or("");
        let mut response = self.request(method, message["params"].clone())?;
        response["id"] = message["id"].clone();
        if response.get("error").is_some() {
            return protocol::write(&mut self.client, &response);
        }
        if method == "completionItem/resolve" {
            let original = &message["params"];
            if response["result"].is_object() {
                // rust-analyzer is allowed to refresh the primary edit on resolve.
                // Keep our source range and snippet while accepting its imports/docs.
                if decorated(original) {
                    response["result"]["textEdit"] = original["textEdit"].clone();
                    response["result"]["insertTextFormat"] = original["insertTextFormat"].clone();
                    if let Some(additions) = original["additionalTextEdits"].as_array() {
                        for edit in additions {
                            append_edit(&mut response["result"], edit.clone());
                        }
                    }
                } else {
                    macro_snippet(&mut response["result"], self.snippets);
                    if let Some(snapshot) = self.completion_contexts.get(&key(original)).cloned()
                        && let Some(children) = children(&response["result"])
                    {
                        if snapshot.revision != self.revision {
                            response = json!({"jsonrpc":"2.0", "id":message["id"], "error":{"code":-32801,"message":"document changed since completion; retry"}});
                        } else if let Some(text) = self.documents.get(&snapshot.uri) {
                            tag_snippet(
                                &mut response["result"],
                                text,
                                &snapshot.context,
                                children,
                                self.snippets,
                            );
                        }
                    }
                }
            }
        } else {
            self.completion_contexts.clear();
            let text = message["params"]["textDocument"]["uri"]
                .as_str()
                .and_then(|uri| self.documents.get(uri))
                .cloned();
            let cursor = text
                .as_deref()
                .and_then(|text| source::offset(text, &message["params"]["position"]));
            let context = text.as_deref().and_then(|text| {
                source::offset(text, &message["params"]["position"])
                    .and_then(|offset| Context::find(text, offset))
            });
            if let (Some(text), Some(cursor)) = (text.as_deref(), cursor) {
                let mut extras = crate::stylesheet::rust_completions(text, cursor);
                for item in &mut extras {
                    item["data"]["revision"] = json!(revision);
                }
                if !extras.is_empty() {
                    if response["result"].is_null() {
                        response["result"] = json!([]);
                    }
                    if let Some(items) = response["result"].as_array_mut() {
                        items.extend(extras);
                    } else if let Some(items) = response["result"]["items"].as_array_mut() {
                        items.extend(extras);
                    }
                }
            }
            let items = if response["result"].is_array() {
                response["result"].as_array_mut()
            } else {
                response["result"]
                    .get_mut("items")
                    .and_then(Value::as_array_mut)
            };
            if let Some(items) = items {
                for item in items {
                    let tag = context.as_ref().is_some_and(|context| !context.closing)
                        && item["kind"] == 22;
                    let view_macro = (item["kind"] == 3 || item["kind"] == 14)
                        && item["label"].as_str().is_some_and(|label| {
                            label.contains('!')
                                || label == "stylesheet"
                                || label.starts_with("stylesheet!")
                                || label.starts_with("stylesheet(")
                                || label == "view"
                                || label.starts_with("view!")
                                || label.starts_with("view(")
                        });
                    let matching_tag = tag
                        && context
                            .as_ref()
                            .zip(text.as_deref())
                            .zip(cursor)
                            .is_some_and(|((context, text), cursor)| {
                                let prefix = text.get(context.name.start..cursor).unwrap_or("");
                                !prefix.is_empty() && matches_prefix(item, prefix)
                            });
                    if (matching_tag || view_macro)
                        && syntax(item).is_none()
                        && !item["data"].is_null()
                    {
                        let resolved = self.request("completionItem/resolve", item.clone())?;
                        if resolved.get("error").is_some() {
                            response = resolved;
                            response["id"] = message["id"].clone();
                            return protocol::write(&mut self.client, &response);
                        }
                        if resolved["result"].is_object() {
                            *item = resolved["result"].clone();
                        }
                    }
                    if tag && let Some(context) = &context {
                        self.completion_contexts.insert(
                            key(item),
                            Snapshot {
                                uri: message["params"]["textDocument"]["uri"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_owned(),
                                revision,
                                context: context.clone(),
                            },
                        );
                    }
                    macro_snippet(item, self.snippets);
                    if tag
                        && let (Some(text), Some(context), Some(children)) =
                            (text.as_deref(), context.as_ref(), children(item))
                    {
                        tag_snippet(item, text, context, children, self.snippets);
                    }
                    if self.cancelled || self.revision != revision {
                        break;
                    }
                }
            }
        }
        if self.cancelled {
            response = json!({"jsonrpc":"2.0", "id":message["id"], "error":{"code":-32800,"message":"request cancelled"}});
        } else if self.revision != revision {
            response = json!({"jsonrpc":"2.0", "id":message["id"], "error":{"code":-32801,"message":"document changed during completion; retry"}});
        }
        protocol::write(&mut self.client, &response)
    }
}

/// Returns a local cache key while keeping the upstream data untouched.
///
/// # Arguments
///
/// * `item` — Original rust-analyzer completion item.
///
/// # Returns
///
/// A stable key for this item and its request-specific semantic identity.
fn key(item: &Value) -> String {
    json!([
        item["data"],
        item["label"],
        item["filterText"],
        item["textEdit"]
            .get("range")
            .or_else(|| item["textEdit"].get("replace"))
    ])
    .to_string()
}

/// Returns whether a candidate can match the user's current completion prefix.
///
/// # Arguments
///
/// * `item` — Candidate completion item.
/// * `prefix` — Typed source prefix.
///
/// # Returns
///
/// A case-insensitive subsequence match, leaving other candidates for lazy resolve.
fn matches_prefix(item: &Value, prefix: &str) -> bool {
    let label = item["filterText"]
        .as_str()
        .or_else(|| item["label"].as_str())
        .unwrap_or("");
    let mut chars = label.chars().flat_map(char::to_lowercase);
    prefix
        .chars()
        .flat_map(char::to_lowercase)
        .all(|expected| chars.any(|candidate| candidate == expected))
}

/// Returns the final generated view-syntax documentation section.
///
/// # Arguments
///
/// * `item` — Semantically resolved rust-analyzer completion item.
///
/// # Returns
///
/// An optional stable contract from the original component or macro declaration.
fn syntax(item: &Value) -> Option<&str> {
    let docs = item["documentation"]
        .as_str()
        .or_else(|| item["documentation"]["value"].as_str())?;
    docs.rsplit_once("# View syntax\n\n")
        .map(|(_, syntax)| syntax.trim())
}

/// Returns the nested-child contract carried by the resolved component symbol.
///
/// # Arguments
///
/// * `item` — Resolved completion item.
///
/// # Returns
///
/// An optional paired-tag preference; unrelated Rust types return [`None`].
fn children(item: &Value) -> Option<bool> {
    match syntax(item)? {
        "Accepts nested child content." => Some(true),
        "Does not accept nested child content." => Some(false),
        _ => None,
    }
}

/// Normalizes the spacing of Leptatui's native brace-delimited macro snippet.
///
/// # Arguments
///
/// * `item` — Completion item to normalize in place.
/// * `snippets` — Whether the client supports LSP snippets.
fn macro_snippet(item: &mut Value, snippets: bool) {
    if !matches!(
        syntax(item),
        Some("Use brace-delimited terminal markup." | "Use brace-delimited terminal stylesheets.")
    ) {
        return;
    }
    let Some(text) = item["textEdit"]["newText"].as_str() else {
        return;
    };
    let name = if let Some(name) = text
        .strip_suffix("! {$0}")
        .or_else(|| text.strip_suffix("!($0)"))
    {
        name
    } else if !snippets {
        let Some(name) = text.strip_suffix('!') else {
            return;
        };
        name
    } else {
        return;
    };
    item["textEdit"]["newText"] = json!(if snippets {
        format!("{name}! {{ $0 }}")
    } else {
        format!("{name}! {{ }}")
    });
    item["insertTextFormat"] = json!(if snippets { 2 } else { 1 });
}

/// Inserts a complete tag while preserving existing delimiters and child content.
///
/// # Arguments
///
/// * `item` — Completion item to decorate.
/// * `text` — Original document.
/// * `context` — Source tag being completed.
/// * `children` — Whether the component accepts nested content.
/// * `snippets` — Whether the client supports snippet tab stops.
fn tag_snippet(item: &mut Value, text: &str, context: &Context, children: bool, snippets: bool) {
    let Some(name) = item["filterText"]
        .as_str()
        .or_else(|| item["label"].as_str())
    else {
        return;
    };
    if syn::parse_str::<syn::Ident>(name).is_err() {
        return;
    }
    let name = name.to_owned();
    let cursor = if snippets { "$0" } else { "" };
    let attrs = text[context.attributes.clone()].trim_end();
    let attrs = if snippets {
        escape(attrs)
    } else {
        attrs.to_owned()
    };
    let closing = context.matching_close(text, &name);
    let new_text = if closing.is_some() {
        format!("<{name}{attrs}>{cursor}")
    } else if children {
        format!("<{name}{attrs}>{cursor}</{name}>")
    } else if snippets {
        format!("<{name}{attrs} {cursor} />")
    } else {
        format!("<{name}{attrs} />")
    };
    item["textEdit"] = json!({
        "range": range(text, context.opening.unwrap_or(context.name.start)..context.end),
        "newText": new_text,
    });
    item["insertTextFormat"] = json!(if snippets { 2 } else { 1 });
    if let Some(object) = item.as_object_mut() {
        object.remove("insertText");
        object.remove("textEditText");
    }
    if let Some(closing) = closing {
        append_edit(item, json!({"range":range(text, closing),"newText":name}));
    }
}

/// Escapes source text copied into an LSP snippet without interpreting variables.
///
/// # Arguments
///
/// * `text` — Literal attributes to preserve.
///
/// # Returns
///
/// A string with snippet metacharacters escaped.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('$', "\\$")
        .replace('}', "\\}")
}

/// Appends an independent edit without duplicating rust-analyzer import edits.
///
/// # Arguments
///
/// * `item` — Completion item being decorated.
/// * `edit` — Additional plain-text edit to retain.
fn append_edit(item: &mut Value, edit: Value) {
    if !item["additionalTextEdits"].is_array() {
        item["additionalTextEdits"] = json!([]);
    }
    if let Some(edits) = item["additionalTextEdits"].as_array_mut()
        && !edits.contains(&edit)
    {
        edits.push(edit);
    }
}

/// Returns whether an item already carries a proxy-generated completion edit.
///
/// # Arguments
///
/// * `item` — Completion item returned to the client.
///
/// # Returns
///
/// A [`bool`] identifying markup snippets and the normalized macro snippet.
fn decorated(item: &Value) -> bool {
    let text = item["textEdit"]["newText"].as_str().unwrap_or("");
    (children(item).is_some() && text.starts_with('<'))
        || (matches!(
            syntax(item),
            Some(
                "Use brace-delimited terminal markup."
                    | "Use brace-delimited terminal stylesheets."
            )
        ) && text.contains("! { "))
}

/// Converts a byte range into a UTF-16 LSP text range.
///
/// # Arguments
///
/// * `text` — Original document.
/// * `bytes` — Source byte range.
///
/// # Returns
///
/// A JSON LSP range using UTF-16 character offsets.
fn range(text: &str, bytes: Range<usize>) -> Value {
    /// Converts one source byte offset into an LSP position.
    ///
    /// # Arguments
    ///
    /// * `text` — Source document.
    /// * `offset` — Byte offset at a character boundary.
    ///
    /// # Returns
    ///
    /// A UTF-16 LSP position.
    fn position(text: &str, offset: usize) -> Value {
        let prefix = &text[..offset];
        json!({"line":prefix.bytes().filter(|byte| *byte == b'\n').count(),
            "character":prefix.rsplit('\n').next().unwrap_or("").encode_utf16().count()})
    }
    json!({"start":position(text, bytes.start), "end":position(text, bytes.end)})
}

#[cfg(test)]
mod tests {
    //! Text-edit and snippet regressions independent of the language server.

    use super::*;

    /// Creates a resolved component item with an opaque upstream identity.
    ///
    /// # Arguments
    ///
    /// * `name` — Name visible in the user's module.
    ///
    /// # Returns
    ///
    /// A completion item accepting nested children.
    fn component(name: &str) -> Value {
        json!({"label":name,"filterText":name,"data":{"opaque":[1,2,3]},
            "documentation":{"kind":"markdown","value":"A component.\n\n# View syntax\n\nAccepts nested child content."}})
    }

    /// Verifies literal attributes and upstream identity survive snippet generation.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// view! { <For class={"$PATH\\file"}><Text>"Body"</Text></For> }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Copied attributes escape snippet metacharacters without losing source.
    /// - Child content is not replaced and the existing closing name is updated.
    /// - Opaque rust-analyzer data remains unchanged.
    #[test]
    fn snippets_preserve_existing_source_and_data() {
        let text = "view! { <For class={\"$PATH\\\\file\"}><Text>\"Body\"</Text></For> }";
        let context = Context::find(text, text.find("For").unwrap() + 3).unwrap();
        let mut item = component("Form");
        let data = item["data"].clone();
        tag_snippet(&mut item, text, &context, true, true);
        assert_eq!(
            item["textEdit"]["newText"],
            "<Form class={\"\\$PATH\\\\\\\\file\"\\}>$0"
        );
        assert_eq!(item["data"], data);
        assert_eq!(item["additionalTextEdits"][0]["newText"], "Form");
        assert!(decorated(&item));
    }

    /// Verifies non-snippet clients receive usable plain tags without tab stops.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// view! { Inp }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Self-closing tags retain the requested spacing and format is plain text.
    #[test]
    fn plain_text_fallback_has_no_tab_stop() {
        let text = "view! { Inp }";
        let context = Context::find(text, text.find("Inp").unwrap() + 3).unwrap();
        let mut item = component("Input");
        tag_snippet(&mut item, text, &context, false, false);
        assert_eq!(item["textEdit"]["newText"], "<Input />");
        assert_eq!(item["insertTextFormat"], 1);
    }
    /// Verifies stylesheet macro snippets preserve aliases and upstream edit identity.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// stylesheet!($0)
    /// styles! {$0}
    /// stylesheet!
    /// ```
    ///
    /// # Assertions
    ///
    /// - Snippet and plain-text clients receive brace-delimited macros.
    /// - Upstream opaque data and insert/replace ranges are retained.
    #[test]
    fn stylesheet_macro_snippets_preserve_identity() {
        for (source, snippets, expected) in [
            ("stylesheet!($0)", true, "stylesheet! { $0 }"),
            ("styles! {$0}", true, "styles! { $0 }"),
            ("stylesheet!", false, "stylesheet! { }"),
        ] {
            let mut item = json!({"label":"stylesheet!","data":{"opaque":123},"documentation":"# View syntax\n\nUse brace-delimited terminal stylesheets.","textEdit":{"newText":source,"insert":{"start":{"line":0,"character":0},"end":{"line":0,"character":3}},"replace":{"start":{"line":0,"character":0},"end":{"line":0,"character":5}}}});
            let identity = item["data"].clone();
            let insert = item["textEdit"]["insert"].clone();
            let replace = item["textEdit"]["replace"].clone();
            macro_snippet(&mut item, snippets);
            assert_eq!(item["textEdit"]["newText"], expected);
            assert_eq!(item["data"], identity);
            assert_eq!(item["textEdit"]["insert"], insert);
            assert_eq!(item["textEdit"]["replace"], replace);
            assert!(decorated(&item));
        }
    }
}
