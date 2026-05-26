use std::collections::hash_map::Entry;
use std::fs;
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::ops::ControlFlow;
use std::process::Stdio;
use std::sync::LazyLock;
use std::{collections::HashMap, sync::RwLock};

use async_channel::{Receiver, Sender};
use async_lsp::LanguageServer;
use async_lsp::ServerSocket;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::lsp_types::notification::{Progress, PublishDiagnostics, ShowMessage};
use async_lsp::lsp_types::{
    ClientCapabilities, DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, InitializeParams, InitializedParams, ProgressParamsValue,
    PublishDiagnosticsParams, TextDocumentIdentifier, TextDocumentItem, Url,
    WindowClientCapabilities, WorkDoneProgress, WorkspaceFolder, WorkspaceFoldersChangeEvent,
};
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::tracing::TracingLayer;
use tokio::runtime::Runtime;
use tower::ServiceBuilder;

use crate::config::{self, Config};

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
    let (mainloop, mut server) = async_lsp::MainLoop::new_client(|_server| {
        let mut router = Router::new(ClientState);
        router
            .notification::<Progress>(|_this, prog| {
                // println!("{:?} {:?}", prog.token, prog.value);
                match prog.value {
                    ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(_)) => loading::add(1),
                    ProgressParamsValue::WorkDone(WorkDoneProgress::End(_)) => loading::sub(1),
                    _ => (),
                }
                ControlFlow::Continue(())
            })
            .notification::<PublishDiagnostics>(|_, prog| {
                DIAG_SENDERS.write().unwrap().iter_mut().for_each(|sender| {
                    sender.send_blocking(prog.clone()).unwrap();
                });
                ControlFlow::Continue(())
            })
            .notification::<ShowMessage>(|_, params| {
                println!("Message {:?}: {}", params.typ, params.message);
                ControlFlow::Continue(())
            })
            .event(|_, _: Stop| ControlFlow::Break(Ok(())));

        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .service(router)
    });

    let command_full = command.to_string();
    let mut command = command.split(" ").collect::<Vec<&str>>();
    let child = async_process::Command::new(command[0])
        .current_dir(workspace_uri)
        .args(command.split_off(1))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) // TODO read pipe, print in log file
        .spawn()?;
    let stdout = child.stdout.unwrap();
    let stdin = child.stdin.unwrap();

    eprintln!("Started LSP {command_full}");

    // Spawn main loop
    tokio::spawn(async move {
        match mainloop.run_buffered(stdout, stdin).await {
            Ok(_) => (),
            Err(_) => eprintln!("LSP {command_full} crashed!"),
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
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        })
        .await
        .unwrap();
    server.initialized(InitializedParams {}).unwrap();

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
                        Err(error::Error::LspNotFound(lsp_command))
                    } else {
                        Err(error::Error::LspFailed(lsp_command))
                    }
                }
            },
            Entry::Occupied(mut entry) => {
                let mut server = entry.get_mut().clone();
                server
                    .did_change_workspace_folders(DidChangeWorkspaceFoldersParams {
                        event: WorkspaceFoldersChangeEvent {
                            added: vec![WorkspaceFolder {
                                uri: Url::from_file_path(workspace_uri).unwrap(),
                                name: "root".into(),
                            }],
                            removed: vec![],
                        },
                    })
                    .unwrap();
                Ok(server)
            }
        }
    } else {
        Err(error::Error::LspNotConfigured(uri.to_string()))
    }
}
