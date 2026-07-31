use async_lsp::lsp_types::{
    CodeActionContext, CodeActionParams, CodeActionResponse, CompletionParams, CompletionResponse,
    NumberOrString, PartialResultParams, Position, Range, TextDocumentPositionParams,
    WorkDoneProgressParams,
};

use crate::lsp::*;

pub async fn completion(
    uri: &str,
    workspace_uri: &str,
    position: Position,
) -> Option<CompletionResponse> {
    match get_server(uri, workspace_uri) {
        Ok(mut server) => server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::from_file_path(uri).unwrap(),
                    },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            })
            .await
            .unwrap(),
        Err(error::Error::NotConfigured(_)) => None,
        Err(error::Error::NotFound(_)) => None,
        Err(error) => {
            notifications::push(Notification {
                summary: format!("{error}"),
                priority: notifications::data::Priority::High,
                ..Notification::default()
            });
            None
        }
    }
}

pub async fn actions(
    uri: &str,
    workspace_uri: &str,
    position: Position,
) -> Option<CodeActionResponse> {
    match get_server(uri, workspace_uri) {
        Ok(mut server) => server
            .code_action(CodeActionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(uri).unwrap(),
                },
                range: Range {
                    start: position,
                    end: position,
                },
                context: CodeActionContext {
                    diagnostics: vec![],
                    only: None,
                    trigger_kind: None,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .unwrap(),
        Err(error::Error::NotConfigured(_)) => None,
        Err(error::Error::NotFound(_)) => None,
        Err(error) => {
            notifications::push(Notification {
                summary: format!("{error}"),
                priority: notifications::data::Priority::High,
                ..Notification::default()
            });
            None
        }
    }
}
