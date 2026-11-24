//! Rust-analyzer proxy providing tag snippets and complete component refactoring.
//!
//! # Modules
//!
//! - [`completion`] — Macro and component tag snippets.
//! - [`protocol`] — Framed JSON-RPC transport.
//! - [`source`] — Rust identifier indexing and source position conversion.

mod completion;
mod protocol;
mod source;

use serde_json::{Value, json};
use std::{
    collections::{HashMap, VecDeque},
    io::{self, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
};
use url::Url;

/// Incoming transport messages from either side of the proxy.
enum Incoming {
    /// Editor request, response, or notification.
    Client(Value),
    /// Rust-analyzer request, response, or notification.
    Server(Value),
    /// End of a stream or a framing error.
    Closed,
}

/// Language-server state, including unsaved document overlays.
struct Proxy {
    /// Stream feeding the rust-analyzer process.
    server: std::process::ChildStdin,
    /// Stream feeding the editor.
    client: io::Stdout,
    /// Messages from both input threads.
    incoming: mpsc::Receiver<Incoming>,
    /// Open source documents, including unsaved edits.
    documents: HashMap<String, String>,
    /// Workspace roots advertised by the client.
    roots: Vec<PathBuf>,
    /// Counter for private requests sent to rust-analyzer.
    sequence: u64,
    /// Counter detecting edits received while rename is being computed.
    revision: u64,
    /// Semantic requests queued while an earlier request is being resolved.
    pending: VecDeque<Value>,
    /// Client request currently being augmented.
    active: Option<Value>,
    /// Whether the client cancelled the active request.
    cancelled: bool,
    /// Whether the client can expand LSP snippet tab stops.
    snippets: bool,
    /// Source contexts for deferred tag completions from the latest response.
    completion_contexts: HashMap<String, completion::Snapshot>,
}

impl Proxy {
    /// Sends a private request while continuing to relay unrelated traffic.
    ///
    /// # Arguments
    ///
    /// * `method` — Rust-analyzer LSP method.
    /// * `params` — Request parameters.
    ///
    /// # Returns
    ///
    /// A JSON-RPC response, including server error responses.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if a transport closes or fails.
    fn request(&mut self, method: &str, params: Value) -> io::Result<Value> {
        self.sequence += 1;
        let id = format!("leptatui-internal-{}", self.sequence);
        protocol::write(
            &mut self.server,
            &json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}),
        )?;
        loop {
            match self.incoming.recv().map_err(io::Error::other)? {
                Incoming::Server(message) if message["id"] == id => return Ok(message),
                Incoming::Server(message) => protocol::write(&mut self.client, &message)?,
                Incoming::Client(message) => {
                    if matches!(
                        message["method"].as_str(),
                        Some(
                            "textDocument/rename"
                                | "textDocument/references"
                                | "textDocument/completion"
                                | "completionItem/resolve"
                        )
                    ) {
                        self.pending.push_back(message);
                        continue;
                    }
                    if message["method"] == "$/cancelRequest"
                        && self.active.as_ref() == Some(&message["params"]["id"])
                    {
                        self.cancelled = true;
                        protocol::write(
                            &mut self.server,
                            &json!({
                                "jsonrpc":"2.0", "method":"$/cancelRequest", "params":{"id":id},
                            }),
                        )?;
                    } else {
                        self.observe(&message);
                        protocol::write(&mut self.server, &message)?;
                    }
                }
                Incoming::Closed => return Err(io::ErrorKind::UnexpectedEof.into()),
            }
        }
    }

    /// Tracks workspace roots and open document changes before forwarding them.
    ///
    /// # Arguments
    ///
    /// * `message` — Incoming client message.
    fn observe(&mut self, message: &Value) {
        let params = &message["params"];
        match message["method"].as_str().unwrap_or("") {
            "initialize" => {
                self.snippets = params
                    .pointer("/capabilities/textDocument/completion/completionItem/snippetSupport")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if let Some(folders) = params["workspaceFolders"].as_array() {
                    for folder in folders {
                        self.add_root(&folder["uri"]);
                    }
                }
                if self.roots.is_empty() {
                    self.add_root(&params["rootUri"]);
                }
            }
            "workspace/didChangeWorkspaceFolders" => {
                if let Some(folders) = params["event"]["removed"].as_array() {
                    for folder in folders {
                        if let Some(path) = file_path(&folder["uri"]) {
                            self.roots.retain(|root| *root != path);
                        }
                    }
                }
                if let Some(folders) = params["event"]["added"].as_array() {
                    for folder in folders {
                        self.add_root(&folder["uri"]);
                    }
                }
            }
            "textDocument/didOpen" => {
                if let (Some(uri), Some(text)) = (
                    params["textDocument"]["uri"].as_str(),
                    params["textDocument"]["text"].as_str(),
                ) {
                    self.documents.insert(uri.to_owned(), text.to_owned());
                    self.revision += 1;
                }
            }
            "textDocument/didChange" => {
                if let Some(text) = params["textDocument"]["uri"]
                    .as_str()
                    .and_then(|uri| self.documents.get_mut(uri))
                    && let Some(changes) = params["contentChanges"].as_array()
                {
                    for change in changes {
                        let Some(next) = change["text"].as_str() else {
                            continue;
                        };
                        if change["range"].is_object() {
                            if let (Some(start), Some(end)) = (
                                source::offset(text, &change["range"]["start"]),
                                source::offset(text, &change["range"]["end"]),
                            ) && start <= end
                            {
                                text.replace_range(start..end, next);
                            }
                        } else {
                            *text = next.to_owned();
                        }
                    }
                    self.revision += 1;
                }
            }
            "workspace/didChangeWatchedFiles" => self.revision += 1,
            "textDocument/didClose" => {
                if let Some(uri) = params["textDocument"]["uri"].as_str() {
                    self.documents.remove(uri);
                }
                self.revision += 1;
            }
            _ => {}
        }
    }

    /// Registers a local workspace root.
    ///
    /// # Arguments
    ///
    /// * `uri` — Client workspace URI.
    fn add_root(&mut self, uri: &Value) {
        if let Some(path) = file_path(uri)
            && !self.roots.contains(&path)
        {
            self.roots.push(path);
        }
    }

    /// Completes component and prop renames across symbols sharing a declaration.
    ///
    /// # Arguments
    ///
    /// * `message` — Original client rename request.
    ///
    /// # Returns
    ///
    /// An empty result after the final response is delivered.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if a transport fails.
    fn rename(&mut self, message: Value) -> io::Result<()> {
        self.active = Some(message["id"].clone());
        self.cancelled = false;
        let result = self.augment(message);
        self.active = None;
        result
    }

    /// Merges semantic requests for generated symbols sharing one source declaration.
    ///
    /// # Arguments
    ///
    /// * `message` — Client rename or references request.
    ///
    /// # Returns
    ///
    /// An empty result after delivering the combined response.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the transport fails.
    fn augment(&mut self, message: Value) -> io::Result<()> {
        let references = message["method"] == "textDocument/references";
        let mut params = message["params"].clone();
        if references {
            params["newName"] = json!("__leptatui_reference_probe");
        }
        let revision = self.revision;
        let mut response = self.request("textDocument/rename", params.clone())?;
        response["id"] = message["id"].clone();
        if response.get("error").is_some() || response["result"].is_null() {
            if references {
                response = self.request("textDocument/references", message["params"].clone())?;
                response["id"] = message["id"].clone();
            }
            return protocol::write(&mut self.client, &response);
        }
        let new_name = params["newName"].as_str().unwrap_or("");
        let mut documents = self.documents.clone();
        for root in &self.roots {
            source::documents(root, &mut documents);
        }
        let mut edits = edit_map(&response["result"]);
        let mut symbols = Vec::new();
        for (uri, text) in &documents {
            for symbol in source::component_symbols(text) {
                if edits.get(uri).is_some_and(|edits| {
                    edits.iter().any(|edit| edit["range"] == symbol.ident.range)
                }) {
                    symbols.push((uri.clone(), symbol));
                }
            }
        }
        if symbols.is_empty() {
            if references {
                response = self.request("textDocument/references", message["params"].clone())?;
                response["id"] = message["id"].clone();
            }
            return protocol::write(&mut self.client, &response);
        }
        'documents: for (uri, text) in &documents {
            for ident in source::identifiers(text) {
                for (declaration_uri, symbol) in &symbols {
                    let old_name = &symbol.ident.name;
                    let suffix = if ident.name == *old_name {
                        ""
                    } else if symbol.component {
                        let Some(suffix) = ident.name.strip_prefix(old_name) else {
                            continue;
                        };
                        if !suffix.starts_with("Props") {
                            continue;
                        }
                        suffix
                    } else {
                        continue;
                    };
                    let definition = self.request(
                        "textDocument/definition",
                        json!({
                            "textDocument":{"uri":uri}, "position":ident.range["start"],
                        }),
                    )?;
                    if self.cancelled || self.revision != revision {
                        break 'documents;
                    }
                    if definition.get("error").is_some() {
                        let mut error = definition;
                        error["id"] = message["id"].clone();
                        return protocol::write(&mut self.client, &error);
                    }
                    if !definition_matches(
                        &definition["result"],
                        declaration_uri,
                        &symbol.ident.range,
                    ) {
                        continue;
                    }
                    let rename = self.request("textDocument/rename", json!({
                        "textDocument":{"uri":uri}, "position":ident.range["start"], "newName":format!("{new_name}{suffix}"),
                    }))?;
                    if rename.get("error").is_some() {
                        let mut error = rename;
                        error["id"] = message["id"].clone();
                        return protocol::write(&mut self.client, &error);
                    }
                    for (uri, additions) in edit_map(&rename["result"]) {
                        let target = edits.entry(uri.clone()).or_default();
                        for mut edit in additions {
                            // Generated names share a declaration span, but their
                            // source references retain the Props/Builder suffix.
                            if let Some(text) = documents.get(&uri)
                                && let (Some(start), Some(end)) = (
                                    source::offset(text, &edit["range"]["start"]),
                                    source::offset(text, &edit["range"]["end"]),
                                )
                                && let Some(original) = text.get(start..end)
                            {
                                let original = original.trim_start_matches("r#");
                                if original == old_name {
                                    edit["newText"] = json!(new_name);
                                } else if symbol.component
                                    && let Some(suffix) = original.strip_prefix(old_name)
                                    && suffix.starts_with("Props")
                                {
                                    edit["newText"] = json!(format!("{new_name}{suffix}"));
                                }
                            }
                            if !target.contains(&edit) {
                                target.push(edit);
                            }
                        }
                    }
                }
            }
        }
        if self.cancelled {
            response = json!({"jsonrpc":"2.0", "id":message["id"], "error":{"code":-32800,"message":"request cancelled"}});
        } else if self.revision != revision {
            response = json!({"jsonrpc":"2.0", "id":message["id"], "error":{"code":-32801,"message":"document changed during rename; retry"}});
        } else if references {
            let include_declaration = message["params"]["context"]["includeDeclaration"]
                .as_bool()
                .unwrap_or(false);
            let locations: Vec<_> = edits
                .into_iter()
                .flat_map(|(uri, edits)| {
                    let symbols = &symbols;
                    edits.into_iter().filter_map(move |edit| {
                        let declaration = symbols.iter().any(|(source_uri, symbol)| {
                            *source_uri == uri && symbol.ident.range == edit["range"]
                        });
                        (include_declaration || !declaration)
                            .then(|| json!({"uri":uri, "range":edit["range"]}))
                    })
                })
                .collect();
            response["result"] = json!(locations);
        } else {
            // A single changes map avoids duplicate document entries and version drift.
            response["result"] = json!({"changes":edits});
        }
        protocol::write(&mut self.client, &response)
    }
}

/// Converts a local file URI into a filesystem path.
///
/// # Arguments
///
/// * `uri` — JSON file URI.
///
/// # Returns
///
/// An optional local filesystem path.
fn file_path(uri: &Value) -> Option<PathBuf> {
    Url::parse(uri.as_str()?).ok()?.to_file_path().ok()
}

/// Returns whether a definition points at the original component declaration.
///
/// # Arguments
///
/// * `result` — Definition response, supporting Location and LocationLink forms.
/// * `uri` — Declaration document URI.
/// * `range` — Source declaration name range.
///
/// # Returns
///
/// A [`bool`] indicating whether any returned definition matches.
fn definition_matches(result: &Value, uri: &str, range: &Value) -> bool {
    if let Some(items) = result.as_array() {
        return items
            .iter()
            .any(|item| definition_matches(item, uri, range));
    }
    (result["uri"] == uri && result["range"] == *range)
        || (result["targetUri"] == uri && result["targetSelectionRange"] == *range)
}

/// Normalizes a WorkspaceEdit into document-keyed text edits.
///
/// # Arguments
///
/// * `edit` — WorkspaceEdit returned by rust-analyzer.
///
/// # Returns
///
/// A map of document URIs to their text edits.
fn edit_map(edit: &Value) -> HashMap<String, Vec<Value>> {
    let mut result = HashMap::new();
    if let Some(changes) = edit["changes"].as_object() {
        for (uri, edits) in changes {
            if let Some(edits) = edits.as_array() {
                result.insert(uri.clone(), edits.clone());
            }
        }
    }
    if let Some(changes) = edit["documentChanges"].as_array() {
        for change in changes {
            if let (Some(uri), Some(edits)) = (
                change["textDocument"]["uri"].as_str(),
                change["edits"].as_array(),
            ) {
                result
                    .entry(uri.to_owned())
                    .or_insert_with(Vec::new)
                    .extend(edits.clone());
            }
        }
    }
    result
}

/// Starts the proxy and relays traffic until the editor or server exits.
///
/// # Returns
///
/// An empty result on clean shutdown.
///
/// # Errors
///
/// Returns [`io::Error`] if rust-analyzer cannot start or a transport fails.
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("leptatui-lsp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "leptatui-lsp [rust-analyzer runtime options]\n\nRuns rust-analyzer with Leptatui tag snippets and complete component/prop rename and references.\nSet RUST_ANALYZER to override the rust-analyzer executable."
        );
        return Ok(());
    }
    let mut command =
        Command::new(std::env::var_os("RUST_ANALYZER").unwrap_or_else(|| "rust-analyzer".into()));
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut child = command.spawn()?;
    let server = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("missing server stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing server stdout"))?;
    let (sender, incoming) = mpsc::channel();
    let client_sender = sender.clone();
    thread::spawn(move || {
        let mut reader = io::stdin().lock();
        while let Ok(Some(message)) = protocol::read(&mut reader) {
            if client_sender.send(Incoming::Client(message)).is_err() {
                return;
            }
        }
        let _ = client_sender.send(Incoming::Closed);
    });
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        while let Ok(Some(message)) = protocol::read(&mut reader) {
            if sender.send(Incoming::Server(message)).is_err() {
                return;
            }
        }
        let _ = sender.send(Incoming::Closed);
    });
    let mut proxy = Proxy {
        server,
        client: io::stdout(),
        incoming,
        documents: HashMap::new(),
        roots: Vec::new(),
        sequence: 0,
        revision: 0,
        pending: VecDeque::new(),
        active: None,
        cancelled: false,
        snippets: false,
        completion_contexts: HashMap::new(),
    };
    let result = (|| {
        loop {
            let message = if let Some(message) = proxy.pending.pop_front() {
                Incoming::Client(message)
            } else if let Ok(message) = proxy.incoming.recv() {
                message
            } else {
                break;
            };
            match message {
                Incoming::Client(message) => {
                    proxy.observe(&message);
                    if matches!(
                        message["method"].as_str(),
                        Some("textDocument/rename" | "textDocument/references")
                    ) && message.get("id").is_some()
                    {
                        proxy.rename(message)?;
                    } else if matches!(
                        message["method"].as_str(),
                        Some("textDocument/completion" | "completionItem/resolve")
                    ) && message.get("id").is_some()
                    {
                        proxy.complete(message)?;
                    } else {
                        protocol::write(&mut proxy.server, &message)?;
                    }
                }
                Incoming::Server(message) => protocol::write(&mut proxy.client, &message)?,
                Incoming::Closed => break,
            }
        }
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}
