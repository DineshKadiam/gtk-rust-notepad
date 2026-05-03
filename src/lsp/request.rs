use async_lsp::lsp_types::{
    CompletionParams, CompletionResponse, PartialResultParams, Position,
    TextDocumentPositionParams, WorkDoneProgressParams,
};

use crate::lsp::*;

pub async fn completion(
    uri: &str,
    workspace_uri: &str,
    position: Position,
) -> Option<CompletionResponse> {
    if let Some(mut server) = get_server(uri, workspace_uri) {
        server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::from_file_path(uri).unwrap(),
                    },
                    position: position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            })
            .await
            .unwrap()
    } else {
        None
    }
}
