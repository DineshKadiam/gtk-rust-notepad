use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    /// Error if language server isn't configured
    LspNotConfigured(String),
    /// Error if language server is configured, but not found
    LspNotFound(String),
    /// Error if language server command failed
    LspFailed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::LspNotConfigured(file) => {
                write!(f, "Language server not configured for file \"{}\"", file)
            }
            Error::LspNotFound(lsp) => write!(
                f,
                "Language server \"{}\" not found. Maybe it's not installed?",
                lsp
            ),
            Error::LspFailed(lsp) => write!(f, "Language server command \"{}\" failed.", lsp),
        }
    }
}

impl std::error::Error for Error {}
