use async_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidSaveTextDocumentParams, TextDocumentContentChangeEvent,
    VersionedTextDocumentIdentifier,
};

use super::*;
use crate::lsp::error::Error;

/// Add files to a workspace.
pub fn open(uri: &str, workspace_uri: &str) {
    let (_, language_id) = lookup_lsp(uri);
    match get_server(uri, workspace_uri) {
        Ok(mut server) => server
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: Url::from_file_path(uri).unwrap(),
                    language_id,
                    version: 0,
                    text: fs::read_to_string(uri).unwrap(),
                },
            })
            .unwrap(),
        Err(error) => {
            notifications::push(Notification {
                summary: format!("{error}"),
                priority: notifications::data::Priority::High,
                id: "lsp_error".to_string(),
                ..Notification::default()
            });
        }
    }
}

/// Tell an LSP a file was modified
pub fn change(uri: &str, workspace_uri: &str, new: Option<String>) {
    if let Ok(mut server) = get_server(uri, workspace_uri) {
        server
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: Url::from_file_path(uri).unwrap(),
                    version: 0,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: new.unwrap_or_else(|| fs::read_to_string(uri).unwrap()),
                }],
            })
            .unwrap()
    }
}

/// Tell an LSP a file was saved
pub fn save(uri: &str, workspace_uri: &str) {
    match get_server(uri, workspace_uri) {
        Ok(mut server) => server
            .did_save(DidSaveTextDocumentParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(uri).unwrap(),
                },
                text: fs::read_to_string(uri).unwrap().into(),
            })
            .unwrap(),
        Err(Error::NotConfigured(_)) => (),
        Err(error) => {
            notifications::push(Notification {
                summary: format!("{error}"),
                priority: notifications::data::Priority::High,
                id: "lsp_error".to_string(),
                ..Notification::default()
            });
        }
    }
}

/// Tell an LSP to stop caring about a file
pub fn close(uri: &str, workspace_uri: &str) {
    match get_server(uri, workspace_uri) {
        // TODO there should be a better way to do a match pattern?
        Ok(mut server) => match server.did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(uri).unwrap(),
            },
        }) {
            Ok(_) => (),
            Err(_) => {
                notifications::push(Notification {
                    summary: "LSP did_close failed".to_string(),
                    priority: notifications::data::Priority::High,
                    id: "lsp_error".to_string(),
                    ..Notification::default()
                });
            }
        },
        Err(error) => {
            notifications::push(Notification {
                summary: format!("{error}"),
                priority: notifications::data::Priority::High,
                id: "lsp_error".to_string(),
                ..Notification::default()
            });
        }
    }
}
