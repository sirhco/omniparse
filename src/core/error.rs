//! Error types for Omniparse
//!
//! This module defines the error types used throughout the Omniparse library.
//! All errors implement the standard `Error` trait and provide detailed context
//! about what went wrong during extraction operations.
//!
//! # Examples
//!
//! ```
//! use omniparse::{extract_from_path, Error};
//!
//! match extract_from_path("unknown.xyz") {
//!     Ok(result) => println!("Success: {}", result.mime_type),
//!     Err(Error::UnsupportedFormat(mime)) => {
//!         println!("Format {} is not supported", mime);
//!     }
//!     Err(Error::Io(e)) => {
//!         println!("IO error: {}", e);
//!     }
//!     Err(e) => println!("Other error: {}", e),
//! }
//! ```

use thiserror::Error;

use super::result::ExtractionResult;

/// Main error type for Omniparse operations
///
/// This enum represents all possible errors that can occur during file type detection
/// and content extraction. Each variant provides specific context about the failure.
///
/// # Examples
///
/// ```
/// use omniparse::{extract_from_path, Error};
///
/// let result = extract_from_path("nonexistent.pdf");
/// assert!(matches!(result, Err(Error::Io(_))));
/// ```
#[derive(Debug, Error)]
pub enum Error {
    /// IO error occurred during file operations
    ///
    /// This error is returned when file system operations fail, such as when
    /// a file cannot be opened, read, or when permissions are insufficient.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// The file format is not supported
    ///
    /// This error indicates that the detected MIME type does not have a registered
    /// parser. Use `supported_mime_types()` to check which formats are available.
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::{Error, is_mime_supported};
    ///
    /// if !is_mime_supported("application/x-custom") {
    ///     // This would result in UnsupportedFormat error
    ///     println!("Custom format not supported");
    /// }
    /// ```
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    /// The file appears to be corrupted or malformed
    ///
    /// This error is returned when a file's structure is invalid or doesn't conform
    /// to the expected format specification, even though the type was correctly detected.
    #[error("Corrupted file: {0}")]
    CorruptedFile(String),
    
    /// Error occurred while parsing the file
    ///
    /// This is a general parsing error that occurs when the parser encounters
    /// unexpected data or cannot process the file content.
    #[error("Parse error: {0}")]
    ParseError(String),
    
    /// Failed to detect the file type
    ///
    /// This error is returned when the type detection system cannot determine
    /// the file's MIME type using magic bytes, content analysis, or extension.
    #[error("Detection failed: {0}")]
    DetectionFailed(String),
    
    /// Partial extraction succeeded but with errors
    ///
    /// This error indicates that some content was successfully extracted, but
    /// the operation encountered errors. The partial result is included in the error
    /// so that calling code can decide whether to use the incomplete data.
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::{extract_from_path, Error};
    ///
    /// match extract_from_path("partially_corrupt.pdf") {
    ///     Err(Error::PartialExtraction { message, partial_result }) => {
    ///         println!("Warning: {}", message);
    ///         println!("Partial content: {:?}", partial_result.content);
    ///         // Decide whether to use partial_result
    ///     }
    ///     Ok(result) => println!("Full extraction succeeded"),
    ///     Err(e) => println!("Complete failure: {}", e),
    /// }
    /// ```
    #[error("Partial extraction: {message}, extracted: {partial_result:?}")]
    PartialExtraction {
        /// Description of what went wrong
        message: String,
        /// The partially extracted result
        partial_result: Box<ExtractionResult>,
    },
}

/// Result type alias for Omniparse operations
///
/// This is a convenience type alias that uses `omniparse::Error` as the error type.
/// Most functions in the library return this type.
///
/// # Examples
///
/// ```
/// use omniparse::Result;
///
/// fn my_extraction_function() -> Result<String> {
///     let result = omniparse::extract_from_path("file.txt")?;
///     Ok(format!("Extracted from {}", result.mime_type))
/// }
/// ```
pub type Result<T> = std::result::Result<T, Error>;
