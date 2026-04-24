use std::fmt::Debug;

use async_lsp::{
    LanguageServer,
    lsp_types::{
        DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
        DocumentFilter, PartialResultParams, TextDocumentIdentifier, Url, WorkDoneProgressParams,
        WorkspaceDiagnosticParams,
    },
};

pub async fn diagnostic(uri: &str, workspace_uri: &str) -> DocumentDiagnosticReport {
    let mut server = super::get_server(uri, workspace_uri);
    let file_uri = Url::from_file_path(uri).unwrap();
    let result = server
        .document_diagnostic(DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier {
                uri: file_uri.clone(),
            },
            identifier: None,
            previous_result_id: Some("rust-analyzer".to_string()),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        })
        .await;
    match result {
        Ok(DocumentDiagnosticReportResult::Report(response)) => response,
        Ok(_) => panic!("LSP request mangled?"), // FIXME
        Err(err) => panic!("LSP request failed: {err}"),
    }
}
