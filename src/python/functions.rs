//! Python-callable functions
//!
//! This module provides Python-callable wrapper functions for Omniparse's
//! core extraction functionality. All functions properly release the GIL
//! during I/O and parsing operations to enable concurrent processing.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use crate::python::types::PyExtractionResult;
use crate::python::errors::map_rust_error;

/// Extract content and metadata from a file path.
///
/// This function reads a file from the filesystem, automatically detects its type,
/// and extracts content and metadata using the appropriate parser.
///
/// The Python Global Interpreter Lock (GIL) is released during file I/O and parsing
/// operations, allowing concurrent extraction from multiple threads.
///
/// # Arguments
///
/// * `py` - Python interpreter context
/// * `path` - Path to the file to extract content from
///
/// # Returns
///
/// Returns a `PyExtractionResult` containing:
/// - The detected MIME type
/// - Extracted content (text or binary)
/// - Metadata fields
/// - Detection confidence score
///
/// # Errors
///
/// Raises Python exceptions:
/// - `IOError` - If the file cannot be read
/// - `ValueError` - If the format is unsupported or the file is corrupted
/// - `RuntimeError` - If detection or parsing fails
///
/// # Examples
///
/// ```python
/// import omniparse
///
/// result = omniparse.extract_from_path("document.pdf")
/// print(f"Type: {result.mime_type}")
/// print(f"Content: {result.content}")
/// print(f"Metadata: {result.metadata}")
/// ```
#[pyfunction]
#[pyo3(
    signature = (path),
    text_signature = "(path, /)",
    name = "extract_from_path"
)]
#[doc = "Extract content and metadata from a file at the given path.

This function automatically detects the file format and uses the appropriate
parser to extract text content and metadata. The GIL is released during I/O
and parsing operations, enabling concurrent processing from multiple threads.

Args:
    path (str): Path to the file to extract from

Returns:
    ExtractionResult: Object containing mime_type, content, metadata, and detection_confidence

Raises:
    IOError: If the file cannot be read or does not exist
    ValueError: If the file format is unsupported or the file is corrupted
    RuntimeError: If detection fails or extraction is only partially successful

Example:
    >>> import omniparse
    >>> result = omniparse.extract_from_path(\"document.pdf\")
    >>> print(f\"Type: {result.mime_type}\")
    >>> print(f\"Content: {result.content}\")
    >>> print(f\"Author: {result.metadata.get('author', 'Unknown')}\")
"]
pub fn extract_from_path(py: Python, path: String) -> PyResult<PyExtractionResult> {
    // Release GIL during I/O and parsing operations
    py.allow_threads(|| {
        let result = crate::extract_from_path(&path)
            .map_err(map_rust_error)?;
        
        Ok(PyExtractionResult { inner: result })
    })
}

/// Extract content and metadata from byte data.
///
/// This function extracts content from in-memory byte data. You can optionally
/// provide a MIME type hint to skip type detection and use a specific parser.
///
/// The Python Global Interpreter Lock (GIL) is released during parsing operations,
/// allowing concurrent extraction from multiple threads.
///
/// # Arguments
///
/// * `py` - Python interpreter context
/// * `data` - Byte data to extract content from
/// * `mime_hint` - Optional MIME type hint (e.g., "application/pdf")
///
/// # Returns
///
/// Returns a `PyExtractionResult` with extracted content and metadata.
///
/// # Errors
///
/// Raises Python exceptions:
/// - `ValueError` - If the format is unsupported or the data is corrupted
/// - `RuntimeError` - If detection or parsing fails
///
/// # Examples
///
/// ```python
/// import omniparse
///
/// # Extract with automatic type detection
/// with open("file.json", "rb") as f:
///     data = f.read()
/// result = omniparse.extract_from_bytes(data)
///
/// # Extract with MIME type hint
/// result = omniparse.extract_from_bytes(data, mime_hint="application/json")
/// ```
#[pyfunction]
#[pyo3(
    signature = (data, mime_hint=None),
    text_signature = "(data, mime_hint=None, /)",
    name = "extract_from_bytes"
)]
#[doc = "Extract content and metadata from raw bytes.

This function is useful when you have file data in memory rather than on disk.
You can optionally provide a MIME type hint to improve detection accuracy. The
GIL is released during parsing operations for concurrent processing.

Args:
    data (bytes): Raw bytes of the file content
    mime_hint (str, optional): MIME type hint to assist detection
        (e.g., \"application/pdf\", \"text/plain\"). Defaults to None.

Returns:
    ExtractionResult: Object containing mime_type, content, metadata, and detection_confidence

Raises:
    ValueError: If the data format is unsupported or corrupted
    RuntimeError: If detection fails or extraction is only partially successful

Example:
    >>> import omniparse
    >>> with open(\"document.pdf\", \"rb\") as f:
    ...     data = f.read()
    >>> result = omniparse.extract_from_bytes(data, mime_hint=\"application/pdf\")
    >>> print(result.mime_type)
    'application/pdf'
"]
pub fn extract_from_bytes(
    py: Python,
    data: &Bound<'_, PyBytes>,
    mime_hint: Option<String>
) -> PyResult<PyExtractionResult> {
    let bytes = data.as_bytes();
    let hint = mime_hint.as_deref();
    
    // Release GIL during parsing operations
    py.allow_threads(|| {
        let result = crate::extract_from_bytes(bytes, hint)
            .map_err(map_rust_error)?;
        
        Ok(PyExtractionResult { inner: result })
    })
}

/// Get a list of all supported MIME types.
///
/// This function returns all MIME types that have registered parsers in the system.
/// You can use this to check what formats are supported before attempting extraction.
///
/// # Returns
///
/// A list of MIME type strings (e.g., ["application/pdf", "text/plain", ...])
///
/// # Examples
///
/// ```python
/// import omniparse
///
/// types = omniparse.supported_mime_types()
/// print(f"Omniparse supports {len(types)} formats")
/// for mime_type in types:
///     print(f"  - {mime_type}")
/// ```
#[pyfunction]
#[pyo3(
    text_signature = "()",
    name = "supported_mime_types"
)]
#[doc = "Get a list of all supported MIME types.

Returns all MIME types that have registered parsers in the system. Use this
to check what formats are supported before attempting extraction.

Returns:
    list[str]: List of MIME type strings that can be parsed by omniparse

Example:
    >>> import omniparse
    >>> formats = supported_mime_types()
    >>> print(f\"Supports {len(formats)} formats\")
    Supports 25 formats
    >>> print(\"PDF supported:\", \"application/pdf\" in formats)
    PDF supported: True
"]
pub fn supported_mime_types() -> Vec<String> {
    crate::supported_mime_types()
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
/// `True` if the MIME type is supported, `False` otherwise
///
/// # Examples
///
/// ```python
/// import omniparse
///
/// if omniparse.is_mime_supported("application/pdf"):
///     print("PDF files are supported!")
///
/// if not omniparse.is_mime_supported("application/x-custom"):
///     print("Custom format is not supported")
/// ```
#[pyfunction]
#[pyo3(
    signature = (mime_type),
    text_signature = "(mime_type, /)",
    name = "is_mime_supported"
)]
#[doc = "Check if a specific MIME type is supported.

This is a convenience function to quickly check if a format can be processed
without needing to iterate through all supported types.

Args:
    mime_type (str): MIME type string to check (e.g., \"application/pdf\")

Returns:
    bool: True if the MIME type is supported, False otherwise

Example:
    >>> import omniparse
    >>> if is_mime_supported(\"application/pdf\"):
    ...     result = extract_from_path(\"document.pdf\")
    >>> else:
    ...     print(\"PDF format not supported\")
"]
pub fn is_mime_supported(mime_type: String) -> bool {
    crate::is_mime_supported(&mime_type)
}
