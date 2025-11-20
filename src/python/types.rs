//! Python type wrappers for Omniparse types
//!
//! This module provides Python-compatible wrappers for Rust types,
//! including conversion functions for complex types like Metadata.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use crate::core::result::{ExtractionResult, Content, Metadata, MetadataValue};

/// Python wrapper for ExtractionResult
///
/// This class represents the result of a content extraction operation,
/// containing the detected MIME type, extracted content, metadata, and
/// detection confidence score.
#[pyclass(name = "ExtractionResult")]
#[doc = "Result of a file extraction operation.

This class encapsulates all information extracted from a file, including the
detected MIME type, content (text or binary), metadata fields, and a confidence
score for the type detection.

Attributes:
    mime_type (str): The detected MIME type (e.g., \"application/pdf\", \"text/plain\")
    content (str | bytes | None): Extracted text content, binary data, or None
    metadata (dict): Dictionary containing file metadata (title, author, dates, etc.)
    detection_confidence (float): Confidence score (0.0-1.0) for MIME type detection

Example:
    >>> import omniparse
    >>> result = omniparse.extract_from_path(\"document.pdf\")
    >>> print(result.mime_type)
    'application/pdf'
    >>> print(result.detection_confidence)
    0.95
    >>> print(result.metadata.get('title', 'Untitled'))
    'My Document'
"]
pub struct PyExtractionResult {
    pub(crate) inner: ExtractionResult,
}

#[pymethods]
impl PyExtractionResult {
    /// Get the detected MIME type
    ///
    /// Returns:
    ///     str: The MIME type (e.g., "application/pdf", "text/plain")
    #[getter]
    #[doc = "The detected MIME type of the file.

Returns:
    str: MIME type string (e.g., \"application/pdf\", \"text/plain\", \"image/jpeg\")

Example:
    >>> result = extract_from_path(\"document.pdf\")
    >>> result.mime_type
    'application/pdf'
"]
    fn mime_type(&self) -> String {
        self.inner.mime_type.clone()
    }
    
    /// Get the extracted content
    ///
    /// Returns:
    ///     str | bytes | None: The extracted content as text, binary data, or None
    #[getter]
    #[doc = "The extracted content from the file.

The type of content returned depends on the file format:
- Text-based formats (text/plain, application/json, text/html, etc.) return str
- Binary formats (images, etc.) return bytes
- Some formats may return None if no content is extractable

Returns:
    str | bytes | None: The extracted content

Example:
    >>> result = extract_from_path(\"document.txt\")
    >>> print(result.content)
    'This is the text content...'
    >>> 
    >>> result = extract_from_path(\"image.png\")
    >>> type(result.content)
    <class 'bytes'>
"]
    fn content(&self, py: Python) -> PyResult<PyObject> {
        content_to_py(py, &self.inner.content)
    }
    
    /// Get the extracted metadata
    ///
    /// Returns:
    ///     dict: Dictionary containing metadata fields
    #[getter]
    #[doc = "Metadata extracted from the file.

The metadata dictionary contains format-specific information about the file.
Common metadata keys include:
- title: Document title
- author: Document author or creator
- created: Creation date (ISO 8601 string)
- modified: Last modification date (ISO 8601 string)
- page_count: Number of pages (for documents)
- word_count: Number of words (for text documents)
- width/height: Dimensions (for images)

The exact fields available depend on the file format and what information
is embedded in the file.

Returns:
    dict: Dictionary with string keys and values of various types
        (str, int, float, bool, list)

Example:
    >>> result = extract_from_path(\"document.pdf\")
    >>> result.metadata
    {'title': 'My Document', 'author': 'John Doe', 'page_count': 10}
    >>> result.metadata.get('author', 'Unknown')
    'John Doe'
"]
    fn metadata(&self, py: Python) -> PyResult<PyObject> {
        metadata_to_dict(py, &self.inner.metadata)
    }
    
    /// Get the detection confidence score
    ///
    /// Returns:
    ///     float: Confidence score between 0.0 and 1.0
    #[getter]
    #[doc = "Confidence score for the MIME type detection.

This score indicates how confident the type detection algorithm is about
the identified MIME type. Higher scores indicate more certainty.

Returns:
    float: Confidence score between 0.0 and 1.0, where:
        - 1.0 indicates highest confidence (e.g., magic bytes match)
        - 0.5-0.9 indicates medium confidence (e.g., file extension match)
        - < 0.5 indicates low confidence (fallback detection)

Example:
    >>> result = extract_from_path(\"document.pdf\")
    >>> result.detection_confidence
    0.95
    >>> if result.detection_confidence > 0.8:
    ...     print(\"High confidence detection\")
"]
    fn detection_confidence(&self) -> f32 {
        self.inner.detection_confidence
    }
    
    /// String representation of the extraction result
    ///
    /// Returns:
    ///     str: A readable representation showing MIME type and confidence
    #[doc = "Return a string representation of the extraction result.

Returns:
    str: A readable string showing the MIME type and confidence score

Example:
    >>> result = extract_from_path(\"document.pdf\")
    >>> repr(result)
    \"ExtractionResult(mime_type='application/pdf', confidence=0.95)\"
    >>> print(result)
    ExtractionResult(mime_type='application/pdf', confidence=0.95)
"]
    fn __repr__(&self) -> String {
        format!(
            "ExtractionResult(mime_type='{}', confidence={:.2})",
            self.inner.mime_type,
            self.inner.detection_confidence
        )
    }
}

/// Convert Rust Content enum to Python object
///
/// Maps Content variants to appropriate Python types:
/// - Content::Text -> str
/// - Content::Binary -> bytes
/// - Content::None -> None
pub(crate) fn content_to_py(py: Python, content: &Content) -> PyResult<PyObject> {
    match content {
        Content::Text(text) => Ok(text.clone().into_py(py)),
        Content::Binary(data) => Ok(data.clone().into_py(py)),
        Content::None => Ok(py.None()),
    }
}

/// Convert Rust Metadata to Python dict
///
/// Iterates through all metadata fields and converts each value
/// to its appropriate Python type.
pub(crate) fn metadata_to_dict(py: Python, metadata: &Metadata) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    
    for key in metadata.keys() {
        if let Some(value) = metadata.get(key) {
            let py_value = metadata_value_to_py(py, value)?;
            dict.set_item(key, py_value)?;
        }
    }
    
    Ok(dict.into())
}

/// Convert Rust MetadataValue to Python object
///
/// Maps MetadataValue variants to appropriate Python types:
/// - Text -> str
/// - Number -> int
/// - Float -> float
/// - DateTime -> str (ISO 8601 format)
/// - Boolean -> bool
/// - List -> list (recursive conversion)
pub(crate) fn metadata_value_to_py(py: Python, value: &MetadataValue) -> PyResult<PyObject> {
    match value {
        MetadataValue::Text(s) => Ok(s.clone().into_py(py)),
        MetadataValue::Number(n) => Ok(n.into_py(py)),
        MetadataValue::Float(f) => Ok(f.into_py(py)),
        MetadataValue::DateTime(dt) => {
            // Convert DateTime to ISO 8601 string for Python
            Ok(dt.to_rfc3339().into_py(py))
        }
        MetadataValue::Boolean(b) => Ok(b.into_py(py)),
        MetadataValue::List(items) => {
            let py_list = PyList::empty_bound(py);
            for item in items {
                py_list.append(metadata_value_to_py(py, item)?)?;
            }
            Ok(py_list.into())
        }
    }
}
