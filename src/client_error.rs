#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("{0}")]
    Message(String),
    /// Parser found no models at all (layout changed again or download failed silently).
    #[error("keine Modelle gefunden: Das Seitenlayout hat sich vermutlich erneut geändert")]
    Empty,
}

/// Client error.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// HTTP error while downloading (release path only).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("parse error: {0}")]
    Parsing(#[from] FetchError),
}

impl From<scraper::error::SelectorErrorKind<'_>> for FetchError {
    fn from(value: scraper::error::SelectorErrorKind<'_>) -> Self {
        FetchError::Message(value.to_string())
    }
}
