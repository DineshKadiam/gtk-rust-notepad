use async_lsp::{
    LanguageServer,
    lsp_types::{
        DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
        PartialResultParams, TextDocumentIdentifier, Url, WorkDoneProgressParams,
        WorkspaceDiagnosticParams, request::WorkspaceDiagnosticRefresh,
    },
};

pub async fn workspace_diagnostic(uri: &str, workspace_uri: &str) {
    let mut server = super::get_server(uri, workspace_uri);
    // server
    //     .workspace_diagnostic(WorkspaceDiagnosticParams {
    //         identifier: None,
    //         previous_result_ids: vec![],
    //         work_done_progress_params: WorkDoneProgressParams::default(),
    //         partial_result_params: PartialResultParams::default(),
    //     })
    //     .await
    //     .unwrap();

    server
        .request::<WorkspaceDiagnosticRefresh>(())
        .await
        .unwrap();
}

// FIXME????
pub async fn file_diagnostic(uri: &str, workspace_uri: &str) -> DocumentDiagnosticReport {
    let mut server = super::get_server(uri, workspace_uri);
    let file_uri = Url::from_file_path(uri).unwrap();
    // Wait to finish loading FIXME
    // if super::loading::is_loading() {
    //     let listener = super::loading::listen();
    //     while let Ok(_) = listener.recv().await
    //         && super::loading::is_loading()
    //     {}
    // }
    //
    // Query
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
