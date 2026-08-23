use std::process::Stdio;

use crate::{
    config::{self, Config},
    lsp::loading,
};

/// Formats documents. This isn't substantial enough to be a part of its own module, so it's a part of the lsp module.
/// Runs an in-place formatter if it exists for a language, and returns true if the formatter was successful.
pub async fn format(uri: &str, workspace_uri: &str) -> bool {
    let config = Config::get_or_default();
    let mut servers = config.lang_server;
    servers.append(&mut config::default_lang_servers());
    for server in servers {
        if uri.ends_with(&server.file_extension) && !server.formatter.is_empty() {
            let mut command = server.formatter.split(" ").collect::<Vec<&str>>();
            loading::add(1);
            let result = async_process::Command::new(command[0])
                .current_dir(workspace_uri)
                .args(command.split_off(1))
                .arg(uri)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .output()
                .await;
            loading::sub(1);
            match result {
                Ok(_) => return true,
                Err(_) => eprintln!("Formatter command failed for {}", uri),
            }
        }
    }
    false
}
