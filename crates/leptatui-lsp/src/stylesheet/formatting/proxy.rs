//! Document formatting requests with upstream composition and revision checks.

use super::*;
use crate::{Incoming, Proxy, protocol};
use std::{io, sync::mpsc, time::Duration};

impl Proxy {
    /// Formats an open Rust document and its complete stylesheet invocations.
    ///
    /// # Arguments
    ///
    /// * `message` — Original whole-document formatting request.
    ///
    /// # Returns
    ///
    /// An empty result after returning edits or a protocol error.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if a protocol transport closes.
    pub(crate) fn format_document(&mut self, message: Value) -> io::Result<()> {
        self.active = Some(message["id"].clone());
        self.cancelled = false;
        let result = self.format_inner(&message);
        self.active = None;
        result
    }

    /// Composes formatting while continuing to observe cancellation and document changes.
    ///
    /// # Arguments
    ///
    /// * `message` — Active document-formatting request.
    ///
    /// # Returns
    ///
    /// An empty result after emitting the final response.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if communication fails.
    fn format_inner(&mut self, message: &Value) -> io::Result<()> {
        let revision = self.revision;
        let uri = message["params"]["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_owned();
        let original = self.documents.get(&uri).cloned();
        let mut response = self.request("textDocument/formatting", message["params"].clone())?;
        response["id"] = message["id"].clone();
        if response.get("error").is_some() {
            return protocol::write(&mut self.client, &response);
        }
        let Some(original) = original else {
            return protocol::write(&mut self.client, &response);
        };
        if !Syntax::new(&original).macros.iter().any(|m| !m.view) {
            return protocol::write(&mut self.client, &response);
        }
        let Some(upstream) = apply(&original, &response["result"]) else {
            response = json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32603,"message":"invalid upstream formatting edits"}});
            return protocol::write(&mut self.client, &response);
        };
        let path = crate::file_path(&json!(uri));
        let lsp = message["params"]["options"].clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let options = expressions::options(path.as_deref(), &lsp);
            let after = document(&original, &upstream, &options, path.as_deref());
            let _ = sender.send(edits(&original, &after));
        });
        loop {
            // Observe already queued client edits before accepting worker output.
            if let Ok(incoming) = self.incoming.try_recv() {
                self.relay_formatting(incoming)?;
                continue;
            }
            if self.cancelled || self.revision != revision {
                break;
            }
            match receiver.try_recv() {
                Ok(result) => {
                    response["result"] = result;
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    response = json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32603,"message":"stylesheet formatter stopped"}});
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
            match self.incoming.recv_timeout(Duration::from_millis(10)) {
                Ok(incoming) => self.relay_formatting(incoming)?,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(io::ErrorKind::UnexpectedEof.into());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
        if self.cancelled {
            response = json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32800,"message":"formatting cancelled"}});
        } else if self.revision != revision {
            response = json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32801,"message":"document changed during formatting; retry"}});
        }
        protocol::write(&mut self.client, &response)
    }

    /// Relays notifications while formatting and queues later client requests.
    ///
    /// # Arguments
    ///
    /// * `incoming` — Message received during a formatter subprocess.
    ///
    /// # Returns
    ///
    /// An empty result after updating state or forwarding the message.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if a transport closes.
    fn relay_formatting(&mut self, incoming: Incoming) -> io::Result<()> {
        match incoming {
            Incoming::Server(message) => self.forward_server(message)?,
            Incoming::Client(message) => {
                if message["method"] == "$/cancelRequest"
                    && self.active.as_ref() == Some(&message["params"]["id"])
                {
                    self.cancelled = true;
                } else if message.get("id").is_some() && message.get("method").is_some() {
                    self.pending.push_back(message);
                } else {
                    self.observe(&message);
                    if matches!(
                        message["method"].as_str(),
                        Some(
                            "textDocument/didOpen"
                                | "textDocument/didChange"
                                | "textDocument/didClose"
                        )
                    ) && let Some(uri) = message["params"]["textDocument"]["uri"].as_str()
                    {
                        self.publish_style_diagnostics(uri)?;
                    }
                    protocol::write(&mut self.server, &message)?;
                }
            }
            Incoming::Closed => return Err(io::ErrorKind::UnexpectedEof.into()),
        }
        Ok(())
    }
}
