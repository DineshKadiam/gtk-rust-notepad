use std::{
    collections::{HashMap, hash_map::Entry},
    fs,
    hash::{BuildHasherDefault, DefaultHasher},
    ops::ControlFlow,
    process::Stdio,
    sync::{LazyLock, RwLock},
};

use async_channel::{Receiver, Sender};
use async_lsp::{
    LanguageServer, ServerSocket,
    concurrency::ConcurrencyLayer,
    lsp_types::{
        ClientCapabilities, CodeActionClientCapabilities, CodeActionKind,
        CodeActionKindLiteralSupport, CodeActionLiteralSupport, CodeActionResponse,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializedParams,
        MessageType, ProgressParamsValue, ProgressToken, PublishDiagnosticsParams,
        TextDocumentClientCapabilities, TextDocumentIdentifier, TextDocumentItem, Url,
        WindowClientCapabilities, WorkDoneProgress, WorkDoneProgressReport, WorkspaceFolder,
        notification::{Progress, PublishDiagnostics, ShowMessage},
        request::{CodeActionRequest, WorkDoneProgressCreate},
    },
    panic::CatchUnwindLayer,
    router::Router,
    tracing::TracingLayer,
};
use futures::io::BufReader;
use gtk::glib::clone;
use tokio::runtime::Runtime;
use tower::ServiceBuilder;

use crate::{
    config::{self, Config},
    notifications,
    notifications::data::Notification,
};

/// Stores language servers globally, and marks their names along with a socket.
/// > [!NOTE]
/// > Security doesn't actually matter because we're just building a convenient lookup table
/// > so it's fine to use a non-random hasher
/// >
static SERVERS: RwLock<HashMap<String, ServerSocket, BuildHasherDefault<DefaultHasher>>> =
    RwLock::new(HashMap::with_hasher(BuildHasherDefault::new()));

/// Tokio runtime for async tasks
static RT: LazyLock<Runtime> = LazyLock::new(|| Runtime::new().unwrap());

/// Keeps track of how many files are loading/being indexed.
pub mod loading;

/// Senders for diagnostics.
type Senders<T> = RwLock<Vec<Sender<T>>>;
pub static DIAG_SENDERS: Senders<PublishDiagnosticsParams> = RwLock::new(Vec::new());

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
    let (tx, rx) = async_channel::unbounded();
    DIAG_SENDERS.write().unwrap().push(tx);
    rx
}

/// Creates a new language server instance (e.g. spawns a command and main loop) and returns a socket to that instance.
/// Most of the code here is taken from the async_lsp example; I should probably clean it up a bit...
async fn new_lsp(workspace_uri: &str, command: &str) -> std::io::Result<ServerSocket> {
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
                .notification::<PublishDiagnostics>(|_, prog| {
                    DIAG_SENDERS.write().unwrap().iter_mut().for_each(|sender| {
                        sender.send_blocking(prog.clone()).unwrap();
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
                .event(|_, _: CodeActionResponse| {
                    println!("what");
                    ControlFlow::Continue(())
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
        .current_dir(workspace_uri)
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
            Err(_) => {
                // Read stderr
                let err = String::from_utf8(
                    BufReader::new(stderr)
                        .buffer()
                        .iter()
                        .copied()
                        .collect::<Vec<u8>>(),
                )
                .unwrap();
                // Send notification
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

pub fn get_server(uri: &str, workspace_uri: &str) -> Result<ServerSocket, error::Error> {
    let (lsp_command, _) = lookup_lsp(uri);
    if !lsp_command.is_empty() {
        let mut a = SERVERS.write().unwrap();
        match a.entry(lsp_command.to_string()) {
            Entry::Vacant(entry) => match RT.block_on(new_lsp(workspace_uri, &lsp_command)) {
                Ok(server) => Ok(entry.insert(server).clone()),
                Err(err) => {
                    if let std::io::ErrorKind::NotFound = err.kind() {
                        Err(error::Error::NotFound(lsp_command))
                    } else {
                        Err(error::Error::Crashed(lsp_command))
                    }
                }
            },
            Entry::Occupied(mut entry) => {
                let server = entry.get_mut().clone();
                // TODO why am I doing this, again?
                // server
                //     .did_change_workspace_folders(DidChangeWorkspaceFoldersParams {
                //         event: WorkspaceFoldersChangeEvent {
                //             added: vec![WorkspaceFolder {
                //                 uri: Url::from_file_path(workspace_uri).unwrap(),
                //                 name: "root".into(),
                //             }],
                //             removed: vec![],
                //         },
                //     })
                //     .unwrap();
                Ok(server)
            }
        }
    } else {
        Err(error::Error::NotConfigured(uri.to_string()))
    }
}
