//! OCR error type. Converts into `core::Error::ParseError` at the boundary.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OcrError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("image decode error: {0}")]
    ImageDecode(String),

    #[error("model unavailable: {0}")]
    ModelUnavailable(String),

    #[error("model checksum mismatch for {file}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("model download failed: {0}")]
    Download(String),

    #[error("preprocess failed: {0}")]
    Preprocess(String),

    #[error("layout analysis failed: {0}")]
    Layout(String),

    #[error("recognition failed: {0}")]
    Recognize(String),

    #[error("engine not configured for this operation: {0}")]
    Config(String),
}

pub type OcrResult<T> = Result<T, OcrError>;

impl From<OcrError> for crate::core::Error {
    fn from(err: OcrError) -> Self {
        crate::core::Error::ParseError(err.to_string())
    }
}
