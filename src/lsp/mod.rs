use std::fs;
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::ops::ControlFlow;
use std::process::Stdio;
use std::sync::LazyLock;
use std::{collections::HashMap, sync::RwLock};

use async_lsp::LanguageServer;
use async_lsp::ServerSocket;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::lsp_types::notification::{Progress, PublishDiagnostics, ShowMessage};
use async_lsp::lsp_types::{
    ClientCapabilities, DidCloseTextDocumentParams, DidOpenTextDocumentParams, HoverContents,
    HoverParams, InitializeParams, InitializedParams, MarkupContent, MessageActionItemCapabilities,
    NumberOrString, Position, ProgressParamsValue, ShowDocumentClientCapabilities,
    ShowMessageRequestClientCapabilities, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Url, WindowClientCapabilities, WorkDoneProgress,
    WorkDoneProgressParams, WorkspaceFolder,
};
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::tracing::TracingLayer;
use tokio::runtime::Runtime;
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

/// Tokio runtime for async tasks
static RT: LazyLock<Runtime> = LazyLock::new(|| Runtime::new().unwrap());

/// Keeps track of how many files are loading/being indexed.
pub mod loading;

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
    let (_, language_id) = lookup_lsp(uri);
    let mut server = get_server(uri, workspace_uri);
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::from_file_path(uri).unwrap(),
                language_id: language_id.into(),
                version: 0,
                text: fs::read_to_string(uri).unwrap().into(),
            },
        })
        .unwrap();
}

/// 2a. Create LSP server
struct ClientState;
struct Stop;

/// Creates a new language server instance (e.g. spawns a command and main loop) and returns a socket to that instance.
/// Most of the code here is taken from the async_lsp example; I should probably clean it up a bit...
async fn new_lsp(workspace_uri: &str, command: &str) -> ServerSocket {
    let (mainloop, mut server) = async_lsp::MainLoop::new_client(|_server| {
        let mut router = Router::new(ClientState);
        router
            .notification::<Progress>(|_this, prog| {
                match prog.value {
                    ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(_)) => loading::add(1),
                    ProgressParamsValue::WorkDone(WorkDoneProgress::End(_)) => loading::sub(1),
                    _ => (),
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
        .spawn()
        .expect(&format!("Failed to run language server {}", command));
    let stdout = child.stdout.unwrap();
    let stdin = child.stdin.unwrap();

    // Spawn main loop
    tokio::spawn(async move {
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

    server
}

/// 3. Request THINGS from an LSP given a file's URI (and workspace URI).
pub mod request;

/// 4. Tell an LSP to stop caring about a file
pub fn close_file(uri: &str, workspace_uri: &str) {
    let mut server = get_server(uri, workspace_uri);
    server
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(uri).unwrap(),
            },
        })
        .unwrap();
}

// /// 5. Close LSP workspace
// pub fn close_workspace(uri: &str) {
//     todo!();
// }
//

// FIXME ! !!!! !
pub fn lookup_lsp(uri: &str) -> (&str, &str) {
    ("rust-analyzer", "rust")
}

pub fn get_server(uri: &str, workspace_uri: &str) -> ServerSocket {
    let (lsp_command, _) = lookup_lsp(uri);
    // Get or create LSP
    let mut a = SERVERS.write().unwrap();
    a.get_mut(workspace_uri)
        .unwrap()
        .entry(lsp_command.to_string())
        .or_insert_with(|| RT.block_on(new_lsp(workspace_uri, lsp_command)))
        .clone()
}
