use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    /// Error if language server isn't configured
    NotConfigured(String),
    /// Error if language server is configured, but not found
    NotFound(String),
    /// Error if language server command failed
    Crashed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NotConfigured(file) => {
                write!(f, "Language server not configured for file \"{}\"", file)
            }
            Error::NotFound(lsp) => write!(
                f,
                "Language server \"{}\" not found. Maybe it's not installed?",
                lsp
            ),
            Error::Crashed(lsp) => write!(f, "Language server command \"{}\" failed.", lsp),
        }
    }
}

impl std::error::Error for Error {}
