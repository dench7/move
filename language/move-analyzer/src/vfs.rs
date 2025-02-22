// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! The language server must operate upon Move source buffers as they are being edited.
//! As a result, it is frequently queried about buffers that have not yet (or may never be) saved
//! to the actual file system.
//!
//! To manage these buffers, this module provides a "virtual file system" -- in reality, it is
//! basically just a mapping from file identifier (this could be the file's path were it to be
//! saved) to its textual contents.

use lsp_server::Notification;
use lsp_types::{
    notification::Notification as _, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams,
};

use crate::symbols;

/// A mapping from identifiers (file names, potentially, but not necessarily) to their contents.
#[derive(Debug, Default)]
pub struct VirtualFileSystem {
    files: std::collections::HashMap<String, String>,
}

impl VirtualFileSystem {
    /// Returns a reference to the buffer corresponding to the given identifier, or `None` if it
    /// is not present in the system.
    pub fn get(&self, identifier: &str) -> Option<&str> {
        self.files.get(identifier).map(|s| s.as_str())
    }

    /// Inserts or overwrites the buffer corresponding to the given identifier.
    ///
    /// TODO: A far more efficient "virtual file system" would update its buffers with changes sent
    /// from the client, instead of completely replacing them each time. The rust-analyzer has a
    /// 'vfs' module that is capable of doing just that, but it is not published on crates.io. If
    /// we could help get it published, we could use it here.
    pub fn update(&mut self, identifier: &str, content: &str) {
        self.files
            .insert(identifier.to_string(), content.to_string());
    }

    /// Removes the buffer and its identifier from the system.
    pub fn remove(&mut self, identifier: &str) {
        self.files.remove(identifier);
    }
}

/// Updates the given virtual file system based on the text document sync notification that was sent.
pub fn on_text_document_sync_notification(
    files: &mut VirtualFileSystem,
    symbolicator_runner: &symbols::SymbolicatorRunner,
    notification: &Notification,
) {
    eprintln!("text document notification");
    match notification.method.as_str() {
        lsp_types::notification::DidOpenTextDocument::METHOD => {
            let parameters =
                serde_json::from_value::<DidOpenTextDocumentParams>(notification.params.clone())
                    .expect("could not deserialize notification");
            files.update(
                parameters.text_document.uri.path(),
                &parameters.text_document.text,
            );
        }
        lsp_types::notification::DidChangeTextDocument::METHOD => {
            let parameters =
                serde_json::from_value::<DidChangeTextDocumentParams>(notification.params.clone())
                    .expect("could not deserialize notification");
            files.update(
                parameters.text_document.uri.path(),
                &parameters.content_changes.last().unwrap().text,
            );
        }
        lsp_types::notification::DidSaveTextDocument::METHOD => {
            let parameters =
                serde_json::from_value::<DidSaveTextDocumentParams>(notification.params.clone())
                    .expect("could not deserialize notification");
            files.update(
                parameters.text_document.uri.path(),
                &parameters.text.unwrap(),
            );
            symbolicator_runner.run();
        }
        lsp_types::notification::DidCloseTextDocument::METHOD => {
            let parameters =
                serde_json::from_value::<DidCloseTextDocumentParams>(notification.params.clone())
                    .expect("could not deserialize notification");
            files.remove(parameters.text_document.uri.path());
        }
        _ => eprintln!("invalid notification '{}'", notification.method),
    }
    eprintln!("text document notification handled");
}
