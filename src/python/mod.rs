//! Python bindings for Omniparse
//!
//! This module provides Python bindings using PyO3, exposing Omniparse's
//! file parsing and metadata extraction capabilities to Python.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

mod types;
mod functions;
mod errors;

use types::PyExtractionResult;
use functions::{extract_from_path, extract_from_bytes, supported_mime_types, is_mime_supported};

/// Python module for Omniparse
///
/// This module exposes the core Omniparse functionality to Python, including:
/// - extract_from_path: Extract content from a file path
/// - extract_from_bytes: Extract content from byte data
/// - supported_mime_types: Get list of supported MIME types
/// - is_mime_supported: Check if a MIME type is supported
/// - ExtractionResult: Class representing extraction results
#[pymodule]
fn _omniparse(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register functions
    m.add_function(wrap_pyfunction!(extract_from_path, m)?)?;
    m.add_function(wrap_pyfunction!(extract_from_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(supported_mime_types, m)?)?;
    m.add_function(wrap_pyfunction!(is_mime_supported, m)?)?;
    
    // Register classes
    m.add_class::<PyExtractionResult>()?;
    
    Ok(())
}
