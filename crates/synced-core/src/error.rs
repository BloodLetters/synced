use thiserror::Error;

/// Defines all operational error conditions in the core engine.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid URL format: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Server returned unexpected status code: {0}")]
    HttpStatus(reqwest::StatusCode),

    #[error("Header extraction failed")]
    HeaderParse,

    #[error("Download operation was cancelled")]
    Cancelled,
}
