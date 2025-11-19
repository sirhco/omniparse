//! Omniparse - A Rust toolkit for detecting and extracting metadata, text, and content from 35+ file formats
//!
//! This library provides both synchronous and asynchronous APIs for content extraction.
//!
//! # Supported Formats
//!
//! - **Text**: Plain text, JSON, CSV, XML, HTML, CSS, RTF
//! - **Documents**: PDF, DOCX, DOC, XLSX, XLS, PPTX, PPT, ODT, ODS, ODP
//! - **Images**: JPEG, PNG, TIFF (metadata only)
//! - **Archives**: ZIP, TAR
//!
//! See [SUPPORTED_FORMATS.md](https://github.com/omniparse/omniparse/blob/main/SUPPORTED_FORMATS.md) for complete details.
//!
//! # Examples
//!
//! ## Basic extraction from a file
//!
//! ```no_run
//! use omniparse::extract_from_path;
//!
//! let result = extract_from_path("document.pdf")?;
//! println!("MIME type: {}", result.mime_type);
//! println!("Content: {:?}", result.content);
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! ## Extract from HTML
//!
//! ```no_run
//! use omniparse::extract_from_path;
//!
//! let result = extract_from_path("webpage.html")?;
//! if let Some(title) = result.metadata.get("title") {
//!     println!("Page title: {:?}", title);
//! }
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! ## Extract from spreadsheets
//!
//! ```no_run
//! use omniparse::extract_from_path;
//!
//! // Works with XLSX, XLS, and ODS
//! let result = extract_from_path("data.xlsx")?;
//! if let Some(sheet_count) = result.metadata.get("sheet_count") {
//!     println!("Number of sheets: {:?}", sheet_count);
//! }
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! ## Extract from bytes with MIME type hint
//!
//! ```no_run
//! use omniparse::extract_from_bytes;
//!
//! let data = std::fs::read("file.json")?;
//! let result = extract_from_bytes(&data, Some("application/json"))?;
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! ## Check supported formats
//!
//! ```
//! use omniparse::{supported_mime_types, is_mime_supported};
//!
//! let types = supported_mime_types();
//! println!("Supported types: {:?}", types);
//!
//! if is_mime_supported("application/pdf") {
//!     println!("PDF is supported!");
//! }
//! ```

pub mod core;
pub mod detection;
pub mod parsers;
pub mod utils;

use std::path::Path;

// Re-export core types for convenience
pub use core::{Error, Result};
pub use core::result::{Content, ExtractionResult, Metadata, MetadataValue};

/// Extract text and metadata from a file at the specified path.
///
/// This function automatically detects the file type using magic bytes and content analysis,
/// then routes the file to the appropriate parser for extraction.
///
/// # Arguments
///
/// * `path` - Path to the file to extract content from
///
/// # Returns
///
/// Returns an `ExtractionResult` containing:
/// - The detected MIME type
/// - Extracted content (text or binary)
/// - Metadata fields (title, author, dates, etc.)
/// - Detection confidence score
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read (IO error)
/// - The file format is not supported
/// - The file is corrupted or malformed
/// - Parsing fails for any reason
///
/// # Examples
///
/// ```no_run
/// use omniparse::extract_from_path;
///
/// let result = extract_from_path("document.pdf")?;
/// println!("Detected type: {}", result.mime_type);
/// 
/// if let omniparse::Content::Text(text) = result.content {
///     println!("Extracted text: {}", text);
/// }
///
/// if let Some(title) = result.metadata.title() {
///     println!("Title: {}", title);
/// }
/// # Ok::<(), omniparse::Error>(())
/// ```
pub fn extract_from_path(path: impl AsRef<Path>) -> Result<ExtractionResult> {
    let extractor = core::Extractor::new();
    extractor.extract_from_path(path)
}

/// Extract text and metadata from a byte slice.
///
/// This function allows extraction from in-memory data. You can optionally provide
/// a MIME type hint to skip type detection and use a specific parser directly.
///
/// # Arguments
///
/// * `data` - Byte slice containing the file data
/// * `mime_hint` - Optional MIME type hint (e.g., "application/pdf")
///
/// # Returns
///
/// Returns an `ExtractionResult` with extracted content and metadata.
///
/// # Errors
///
/// Returns an error if:
/// - The format is not supported
/// - The data is corrupted or malformed
/// - Parsing fails for any reason
///
/// # Examples
///
/// ```no_run
/// use omniparse::extract_from_bytes;
///
/// // Extract with automatic type detection
/// let data = std::fs::read("file.json")?;
/// let result = extract_from_bytes(&data, None)?;
///
/// // Extract with MIME type hint
/// let result = extract_from_bytes(&data, Some("application/json"))?;
/// # Ok::<(), omniparse::Error>(())
/// ```
pub fn extract_from_bytes(data: &[u8], mime_hint: Option<&str>) -> Result<ExtractionResult> {
    let extractor = core::Extractor::new();
    extractor.extract_from_bytes(data, mime_hint)
}

/// Get a list of all supported MIME types.
///
/// This function returns all MIME types that have registered parsers in the system.
/// You can use this to check what formats are supported before attempting extraction.
///
/// # Returns
///
/// A vector of MIME type strings (e.g., "application/pdf", "text/plain")
///
/// # Examples
///
/// ```
/// use omniparse::supported_mime_types;
///
/// let types = supported_mime_types();
/// println!("Omniparse supports {} formats", types.len());
/// for mime_type in types {
///     println!("  - {}", mime_type);
/// }
/// ```
pub fn supported_mime_types() -> Vec<String> {
    let registry = parsers::ParserRegistry::default();
    registry.supported_types()
}

/// Check if a specific MIME type is supported.
///
/// This is a convenience function to quickly check if a format can be processed
/// without needing to iterate through all supported types.
///
/// # Arguments
///
/// * `mime_type` - The MIME type to check (e.g., "application/pdf")
///
/// # Returns
///
/// `true` if the MIME type is supported, `false` otherwise
///
/// # Examples
///
/// ```
/// use omniparse::is_mime_supported;
///
/// if is_mime_supported("application/pdf") {
///     println!("PDF files are supported!");
/// }
///
/// if !is_mime_supported("application/x-custom") {
///     println!("Custom format is not supported");
/// }
/// ```
pub fn is_mime_supported(mime_type: &str) -> bool {
    let registry = parsers::ParserRegistry::default();
    registry.get_parser(mime_type).is_some()
}

/// Extract text and metadata from a file asynchronously.
///
/// This is the async version of `extract_from_path`, using Tokio for async file I/O.
/// It provides the same functionality but allows for non-blocking operation in async contexts.
///
/// **Note:** This function is only available when the `async` feature is enabled.
///
/// # Arguments
///
/// * `path` - Path to the file to extract content from
///
/// # Returns
///
/// Returns an `ExtractionResult` containing the extracted content and metadata.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read (IO error)
/// - The file format is not supported
/// - The file is corrupted or malformed
/// - Parsing fails for any reason
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "async")]
/// # async fn example() -> Result<(), omniparse::Error> {
/// use omniparse::extract_from_path_async;
///
/// let result = extract_from_path_async("document.pdf").await?;
/// println!("Detected type: {}", result.mime_type);
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "async")]
pub async fn extract_from_path_async(path: impl AsRef<Path>) -> Result<ExtractionResult> {
    use tokio::io::AsyncReadExt;
    
    let path = path.as_ref();
    
    // Read the file asynchronously
    let mut file = tokio::fs::File::open(path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    
    // Use the synchronous extraction logic on the buffered data
    // We also need to detect the type from the path
    let extractor = core::Extractor::new();
    let detection = extractor.detector.detect_from_path(path)?;
    
    // Get the parser and parse the data
    let parser = extractor.registry.get_parser(&detection.mime_type)
        .ok_or_else(|| Error::UnsupportedFormat(detection.mime_type.clone()))?;
    
    let mut result = parser.parse(&buffer, &detection.mime_type)?;
    result.detection_confidence = detection.confidence;
    result.mime_type = detection.mime_type;
    
    Ok(result)
}
