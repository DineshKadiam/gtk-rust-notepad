use async_lsp::lsp_types::{
    CodeActionContext, CodeActionKind, CodeActionParams, CodeActionResponse, CodeActionTriggerKind,
    CompletionParams, CompletionResponse, Diagnostic, Hover, HoverParams, PartialResultParams,
    Position, Range, TextDocumentPositionParams, WorkDoneProgressParams,
};

use crate::lsp::*;

pub async fn completion(
    uri: &str,
    workspace_uri: &str,
    position: Position,
) -> Option<CompletionResponse> {
    match get_server_safe(uri, workspace_uri) {
        Some(mut server) => match server
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
        {
            Ok(compl) => compl,
            Err(error) => {
                notifications::push(Notification {
                    summary: format!("{error}"),
                    priority: notifications::data::Priority::High,
                    ..Notification::default()
                });
                None
            }
        },
        None => None,
    }
}

pub async fn hover(uri: &str, workspace_uri: &str, position: Position) -> Option<Hover> {
    match get_server_safe(uri, workspace_uri) {
        Some(mut server) => match server
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::from_file_path(uri).unwrap(),
                    },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
        {
            Ok(hov) => hov,
            Err(error) => {
                notifications::push(Notification {
                    summary: format!("{error}"),
                    priority: notifications::data::Priority::High,
                    ..Notification::default()
                });
                None
            }
        },
        None => None,
    }
}

pub async fn actions(
    uri: &str,
    workspace_uri: &str,
    start: Position,
    end: Position,
    diagnostics: Vec<Diagnostic>,
) -> Option<CodeActionResponse> {
    match get_server_safe(uri, workspace_uri) {
        Some(mut server) => {
            let result = server
                .code_action(CodeActionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::from_file_path(uri).unwrap(),
                    },
                    range: Range { start, end },
                    context: CodeActionContext {
                        diagnostics,
                        only: Some(vec![
                            CodeActionKind::EMPTY,
                            CodeActionKind::QUICKFIX,
                            CodeActionKind::REFACTOR,
                            CodeActionKind::REFACTOR_EXTRACT,
                            CodeActionKind::REFACTOR_INLINE,
                            CodeActionKind::REFACTOR_REWRITE,
                            CodeActionKind::SOURCE,
                            CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
                            CodeActionKind::SOURCE_FIX_ALL,
                        ]),
                        trigger_kind: Some(CodeActionTriggerKind::AUTOMATIC),
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await;
            match result {
                Ok(n) => n,
                Err(e) => {
                    notifications::push(Notification {
                        summary: format!("Code actions failed: {e}"),
                        priority: notifications::data::Priority::High,
                        ..Notification::default()
                    });
                    None
                }
            }
        }
        None => None,
    }
}
