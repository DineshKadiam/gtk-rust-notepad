use std::{
    collections::{HashMap, hash_map::Entry},
    fs,
    ops::ControlFlow,
    process::Stdio,
};

use async_bus::{Bus, Receiver};
use async_lsp::{
    LanguageServer, ServerSocket,
    concurrency::ConcurrencyLayer,
    lsp_types::{
        ClientCapabilities, CodeActionClientCapabilities, CodeActionKindLiteralSupport,
        CodeActionLiteralSupport, CompletionClientCapabilities, CompletionItemCapability,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, HoverClientCapabilities,
        InitializeParams, InitializedParams, InsertTextMode, InsertTextModeSupport, MarkupKind,
        MessageType, ProgressParamsValue, ProgressToken, PublishDiagnosticsParams,
        TextDocumentClientCapabilities, TextDocumentIdentifier, TextDocumentItem, Url,
        WindowClientCapabilities, WorkDoneProgress, WorkDoneProgressReport, WorkspaceFolder,
        notification::{LogMessage, Progress, PublishDiagnostics, ShowMessage},
        request::WorkDoneProgressCreate,
    },
    panic::CatchUnwindLayer,
    router::Router,
    tracing::TracingLayer,
};
use futures::io::BufReader;
use gtk::glib::clone;
use lazy_static::lazy_static;
use tokio::sync::RwLock;
use tower::ServiceBuilder;

use crate::{
    RUNTIME,
    config::{self, Config},
    notifications::{self, data::Notification},
};

lazy_static! {
    /// Stores language servers globally, and marks their names along with a socket.
    /// > [!NOTE]
    /// > Security doesn't actually matter because we're just building a convenient lookup table
    /// > so it's fine to use a non-random hasher
    /// >
    static ref SERVERS: RwLock<HashMap<String, ServerSocket>> = RwLock::new(HashMap::new());
    /// Senders for diagnostics.
    pub static ref DIAG_BUS: Bus<PublishDiagnosticsParams> = Bus::new(64);
}

/// Keeps track of how many files are loading/being indexed.
pub mod loading;

/// File ops (fops 🦊)
pub mod file;

/// Errors...
pub mod error;

// File formatting
pub mod format;

/// 2a. Create LSP server
struct ClientState;
struct Stop;

/// Create listener/sender pair for diagnostics
pub fn listen_diagnostics() -> Receiver<PublishDiagnosticsParams> {
    DIAG_BUS.new_rx()
}

/// Creates a new language server instance (e.g. spawns a command and main loop) and returns a socket to that instance.
/// Most of the code here is taken from the async_lsp example; I should probably clean it up a bit...
async fn new_lsp(workspace_uri: String, command: String) -> std::io::Result<ServerSocket> {
    let (mainloop, mut server) = {
        let command = command.split(" ").collect::<Vec<&str>>()[0].to_string();
        async_lsp::MainLoop::new_client(move |_server| {
            let mut router = Router::new(ClientState);
            router
                .notification::<Progress>(clone!(
                    #[strong]
                    command,
                    move |_this, prog| {
                        let id = format!("{command}_progress_{:?}", prog.token);
                        let summary = if let ProgressToken::String(token_string) = prog.token {
                            token_string.to_string()
                        } else {
                            format!("{command}: loading")
                        };
                        match prog.value {
                            ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(_)) => {}
                            ProgressParamsValue::WorkDone(WorkDoneProgress::End(_)) => {
                                loading::sub(1);
                                notifications::push(Notification {
                                    summary: format!("{summary} (done)"),
                                    status: notifications::data::Status::Done,
                                    id,
                                    ..Notification::default()
                                });
                            }
                            ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                                WorkDoneProgressReport {
                                    message: Some(text),
                                    cancellable: _,
                                    percentage,
                                },
                            )) => {
                                let percentage = percentage.unwrap_or(0);
                                notifications::push(Notification {
                                    summary,
                                    details: text,
                                    status: notifications::data::Status::Progress(
                                        percentage as f32 / 100.,
                                    ),
                                    id,
                                    ..Notification::default()
                                });
                            }
                            _ => (),
                        }
                        ControlFlow::Continue(())
                    }
                ))
                .notification::<LogMessage>(|_this, prog| {
                    // TODO
                    println!("[lsp] {}", prog.message);
                    ControlFlow::Continue(())
                })
                .notification::<PublishDiagnostics>(|_, prog| {
                    RUNTIME.spawn(async move {
                        DIAG_BUS.queue(prog).await;
                    });
                    ControlFlow::Continue(())
                })
                .notification::<ShowMessage>(clone!(
                    #[strong]
                    command,
                    move |_, params| {
                        let priority = match params.typ {
                            MessageType::ERROR => notifications::data::Priority::High,
                            MessageType::WARNING => notifications::data::Priority::Medium,
                            _ => notifications::data::Priority::Low,
                        };
                        notifications::push(Notification {
                            summary: format!("{command}: {:?}", params.typ),
                            details: params.message,
                            id: format!("{command}_message"),
                            priority,
                            ..Notification::default()
                        });
                        ControlFlow::Continue(())
                    }
                ))
                .request::<WorkDoneProgressCreate, _>(|_, _request| async move {
                    loading::add(1);
                    Ok(())
                })
                .event(|_, _: Stop| ControlFlow::Break(Ok(())));

            ServiceBuilder::new()
                .layer(TracingLayer::default())
                .layer(CatchUnwindLayer::default())
                .layer(ConcurrencyLayer::default())
                .service(router)
        })
    };
    let command_full = command.to_string();
    let mut command = command.split(" ").collect::<Vec<&str>>();
    let child = async_process::Command::new(command[0])
        .current_dir(&workspace_uri)
        .args(command.split_off(1))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.unwrap();
    let stdin = child.stdin.unwrap();
    let stderr = child.stderr.unwrap();

    // Spawn main loop
    tokio::spawn(async move {
        match mainloop.run_buffered(stdout, stdin).await {
            Ok(_) => (),
            Err(e) => {
                // Read stderr
                let err = String::from_utf8(BufReader::new(stderr).buffer().to_vec()).unwrap();
                // Send notification
                eprintln!(
                    "[warn] LSP command `{command_full}` crashed with stderr `{err}` and Rust error `{e}`"
                );
                notifications::push(Notification {
                    summary: format!("{command_full} crashed"),
                    details: err,
                    priority: notifications::data::Priority::High,
                    id: format!(
                        "{}_error",
                        command_full.split(" ").collect::<Vec<&str>>()[0]
                    ),
                    ..Notification::default()
                });
            }
        }
        // Remove language server socket from hashmap
        let mut a = SERVERS.write().unwrap();
        a.remove(&command_full);
    });

    // Initialize.
    server
        .initialize(InitializeParams {
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Url::from_file_path(workspace_uri).unwrap(),
                name: "root".into(),
            }]),
            capabilities: ClientCapabilities {
                window: Some(WindowClientCapabilities {
                    work_done_progress: Some(true),
                    ..WindowClientCapabilities::default()
                }),
                text_document: Some(TextDocumentClientCapabilities {
                    code_action: Some(CodeActionClientCapabilities {
                        dynamic_registration: Some(false),
                        code_action_literal_support: Some(CodeActionLiteralSupport {
                            code_action_kind: CodeActionKindLiteralSupport {
                                value_set: vec![
                                    "".to_string(),
                                    "quickfix".to_string(),
                                    "refactor".to_string(),
                                    "refactor.extract".to_string(),
                                    "refactor.inline".to_string(),
                                    "refactor.rewrite".to_string(),
                                    "source".to_string(),
                                    "source.organizeImports".to_string(),
                                    "source.fixAll".to_string(),
                                ],
                            },
                        }),
                        ..CodeActionClientCapabilities::default()
                    }),
                    hover: Some(HoverClientCapabilities {
                        dynamic_registration: Some(false),
                        content_format: Some(vec![MarkupKind::Markdown]),
                    }),
                    completion: Some(CompletionClientCapabilities {
                        dynamic_registration: Some(false),
                        context_support: Some(false),
                        completion_item: Some(CompletionItemCapability {
                            snippet_support: Some(true),
                            documentation_format: Some(vec![MarkupKind::Markdown]),
                            deprecated_support: Some(false),
                            preselect_support: Some(false),
                            insert_replace_support: Some(false),
                            insert_text_mode_support: Some(InsertTextModeSupport {
                                value_set: vec![InsertTextMode::AS_IS],
                            }),
                            label_details_support: Some(true),
                            ..CompletionItemCapability::default()
                        }),
                        ..CompletionClientCapabilities::default()
                    }),
                    ..TextDocumentClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        })
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e))?;
    server
        .initialized(InitializedParams {})
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e))?;

    Ok(server)
}

/// 3. Request THINGS from an LSP given a file's URI (and workspace URI).
pub mod request;

// TODO?
// /// 5. Close LSP workspace
// pub fn close_workspace(uri: &str) {
//     todo!();
// }
//

pub fn lookup_lsp(uri: &str) -> (String, String) {
    let config = Config::get_or_default();
    let mut servers = config.lang_server;
    servers.append(&mut config::default_lang_servers());
    for server in servers {
        if uri.ends_with(&server.file_extension) {
            return (server.command.clone(), server.language.clone());
        }
    }
    (String::new(), String::new())
}

pub async fn get_server(uri: &str, workspace_uri: &str) -> Result<ServerSocket, error::Error> {
    let (lsp_command, _) = lookup_lsp(uri);
    if !lsp_command.is_empty() {
        let mut a = SERVERS.write().await;
        match a.entry(lsp_command.to_string()) {
            Entry::Vacant(entry) => {
                match new_lsp(workspace_uri.to_string(), lsp_command.clone()).await {
                    Ok(server) => Ok(entry.insert(server).clone()),
                    Err(err) => {
                        if let std::io::ErrorKind::NotFound = err.kind() {
                            Err(error::Error::NotFound(lsp_command.clone()))
                        } else {
                            Err(error::Error::Crashed(lsp_command))
                        }
                    }
                }
            }
            Entry::Occupied(mut entry) => {
                let server = entry.get_mut().clone();
                Ok(server)
            }
        }
    } else {
        Err(error::Error::NotConfigured(uri.to_string()))
    }
}

/// Safe wrapper around get_server() which returns an Option of a server and handles errors by printing them with the Notifications API.
/// Used in lsp::request -- lsp::file has its own error handling
pub async fn get_server_safe(uri: &str, workspace_uri: &str) -> Option<ServerSocket> {
    match get_server(uri, workspace_uri).await {
        Ok(server) => Some(server),
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
