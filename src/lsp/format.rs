use std::process::{Command, Stdio};

use crate::config::{self, Config};

/// Formats documents. This isn't substantial enough to be a part of its own module, so it's a part of the lsp module.
/// Runs an in-place formatter if it exists for a language
pub fn format(uri: &str, workspace_uri: &str) {
    let config = Config::get_or_default();
    let mut servers = config.lang_server;
    servers.append(&mut config::default_lang_servers());
    for server in servers {
        if uri.ends_with(&server.file_extension) && !server.formatter.is_empty() {
            let mut command = server.formatter.split(" ").collect::<Vec<&str>>();
            match Command::new(command[0])
                .current_dir(workspace_uri)
                .args(command.split_off(1))
                .arg(uri)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .output()
            {
                Ok(_) => (),
                Err(_) => eprintln!("Formatter command failed for {}", uri),
            }
        }
    }
}
