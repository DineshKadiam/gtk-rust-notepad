use std::hash::{BuildHasherDefault, DefaultHasher};
use std::ops::ControlFlow;
use std::process::Stdio;
use std::{collections::HashMap, sync::RwLock};

use async_channel::{Receiver, Sender};
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::lsp_types::notification::{Progress, PublishDiagnostics, ShowMessage};
use async_lsp::lsp_types::{
    ClientCapabilities, DidOpenTextDocumentParams, HoverContents, HoverParams, InitializeParams,
    InitializedParams, MarkupContent, NumberOrString, Position, ProgressParamsValue,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Url,
    WindowClientCapabilities, WorkDoneProgress, WorkDoneProgressParams, WorkspaceFolder,
};
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::tracing::TracingLayer;
use async_lsp::{Error, ErrorCode, LanguageServer};
use async_lsp::{ServerSocket, tracing};
use tower::ServiceBuilder;

/// Stores language servers globally. There are different language servers per workspace
/// (aka project root aka folder) and language, so we're going to store them in a nested HashMap
/// where we can search the workspace first, and then language.
/// > [!NOTE]
/// > Security doesn't actually matter because we're just building a convenient lookup table
/// > so it's fine to use a non-random hasher
static SERVERS: RwLock<
    HashMap<String, HashMap<String, ServerSocket>, BuildHasherDefault<DefaultHasher>>,
> = RwLock::new(HashMap::with_hasher(BuildHasherDefault::new()));

/// Notifies to windows if all files are indexed. Single global channel (for obvious reasons)
static INDEXED_CHANNEL: RwLock<Option<(Sender<bool>, Receiver<bool>)>> = RwLock::new(None);

/// 0. Get INDEXED_CHANNEL receiver to listen for when all servers load/a new file is added and they have to load again
#[tokio::main(flavor = "current_thread")]
pub async fn get_indexed_listener() -> Receiver<bool> {
    if let Some((_, ref rx)) = *INDEXED_CHANNEL.read().unwrap() {
        rx.clone()
    } else {
        *INDEXED_CHANNEL.write().unwrap() = Some(async_channel::unbounded());
        get_indexed_listener()
    }
}

/// 1. Initialise LSP workspace if it isn't already
pub fn open_workspace(uri: &str) {
    SERVERS
        .write()
        .unwrap()
        .insert(uri.to_string(), HashMap::new());
}

/// 2. Add files to a workspace. Creates an language server for the file's language if it
/// doesn't exist.
pub fn open_file(uri: &str, workspace_uri: &str) {
    todo!();
}

/// 2a. Create LSP server
struct ClientState {
    indexed_tx: Option<Sender<bool>>,
}

struct Stop;

#[tokio::main(flavor = "current_thread")]
async fn new_lsp(workspace_uri: &str, command: &str) -> Receiver<bool> {
    let (tx, rx) = INDEXED_CHANNEL.read().unwrap().clone().unwrap();
    let (mainloop, mut server) = async_lsp::MainLoop::new_client(|_server| {
        let mut router = Router::new(ClientState {
            indexed_tx: Some(tx),
        });
        router
            .notification::<Progress>(|this, prog| {
                println!("{:?} {:?}", prog.token, prog.value);
                if matches!(
                    prog.value,
                    ProgressParamsValue::WorkDone(WorkDoneProgress::End(_))
                ) {
                    if let Some(tx) = this.indexed_tx.take() {
                        tx.send_blocking(false).unwrap();
                    }
                }
                ControlFlow::Continue(())
            })
            .notification::<PublishDiagnostics>(|_, _| ControlFlow::Continue(()))
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

    let child = async_process::Command::new(command)
        .current_dir(workspace_uri)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .expect(&format!("Failed to run language server {}", command));
    let stdout = child.stdout.unwrap();
    let stdin = child.stdin.unwrap();

    let mainloop_fut = tokio::spawn(async move {
        mainloop.run_buffered(stdout, stdin).await.unwrap();
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

    // Start main loop
    tokio::spawn(async move {
        mainloop_fut.await.unwrap();
    });

    rx
}

/// 3. Request THINGS from an LSP given a file's URI (and workspace URI).
/// This is literally the worst template of all lmaooo
pub fn request_thing(uri: &str, workspace_uri: &str, thing: todo!()) -> todo!() {
    todo!();
}

/// 4. Tell an LSP to stop caring about a file
pub fn close_file(uri: &str, workspace_uri: &str) {
    todo!();
}

/// 5. Close LSP workspace
pub fn close_workspace(uri: &str) {
    todo!();
}
