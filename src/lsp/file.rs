use async_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidSaveTextDocumentParams, TextDocumentContentChangeEvent,
    VersionedTextDocumentIdentifier,
};

use super::*;

/// Add files to a workspace.
pub fn open(uri: &str, workspace_uri: &str) {
    let (_, language_id) = lookup_lsp(uri);
    let mut server = get_server(uri, workspace_uri);
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::from_file_path(uri).unwrap(),
                language_id: language_id.into(),
                version: 0,
                text: fs::read_to_string(uri).unwrap(),
            },
        })
        .unwrap();
}

/// Tell an LSP a file was modified
pub fn change(uri: &str, workspace_uri: &str) {
    let mut server = get_server(uri, workspace_uri);
    server
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: Url::from_file_path(uri).unwrap(),
                version: 0,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: fs::read_to_string(uri).unwrap(),
            }],
        })
        .unwrap();
}

/// Tell an LSP a file was saved
pub fn save(uri: &str, workspace_uri: &str) {
    let mut server = get_server(uri, workspace_uri);
    server
        .did_save(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(uri).unwrap(),
            },
            text: fs::read_to_string(uri).unwrap().into(),
        })
        .unwrap();
}

/// Tell an LSP to stop caring about a file
pub fn close(uri: &str, workspace_uri: &str) {
    let mut server = get_server(uri, workspace_uri);
    server
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(uri).unwrap(),
            },
        })
        .unwrap();
}
