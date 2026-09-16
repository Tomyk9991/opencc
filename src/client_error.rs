#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("{0}")]
    Message(String),
}

/// Fehler des Clients.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// HTTP-Fehler beim dynamischen Download (nur Release-Pfad).
    #[error("HTTP-Fehler: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Parsing-Fehler: {0}")]
    Parsing(#[from] FetchError),
}

impl From<scraper::error::SelectorErrorKind<'_>> for FetchError {
    fn from(value: scraper::error::SelectorErrorKind<'_>) -> Self {
        FetchError::Message(value.to_string())
    }
}
