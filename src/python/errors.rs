//! Error conversion from Rust to Python
//!
//! This module provides functions to convert Omniparse Rust errors
//! into appropriate Python exceptions.

use pyo3::prelude::*;
use pyo3::exceptions::{PyIOError, PyValueError, PyRuntimeError};
use crate::core::Error;

/// Convert Rust Error to Python exception
///
/// Maps Omniparse error types to appropriate Python exception types:
/// - Error::Io -> IOError
/// - Error::UnsupportedFormat -> ValueError
/// - Error::CorruptedFile -> ValueError
/// - Error::ParseError -> ValueError
/// - Error::DetectionFailed -> RuntimeError
/// - Error::PartialExtraction -> RuntimeError
///
/// # Arguments
///
/// * `error` - The Rust error to convert
///
/// # Returns
///
/// A PyErr that can be raised in Python
pub(crate) fn map_rust_error(error: Error) -> PyErr {
    match error {
        Error::Io(e) => {
            PyIOError::new_err(format!("IO error: {}", e))
        }
        
        Error::UnsupportedFormat(mime) => {
            PyValueError::new_err(format!("Unsupported format: {}", mime))
        }
        
        Error::CorruptedFile(msg) => {
            PyValueError::new_err(format!("Corrupted file: {}", msg))
        }
        
        Error::ParseError(msg) => {
            PyValueError::new_err(format!("Parse error: {}", msg))
        }
        
        Error::DetectionFailed(msg) => {
            PyRuntimeError::new_err(format!("Detection failed: {}", msg))
        }
        
        Error::PartialExtraction { message, partial_result: _ } => {
            // For partial extraction, we treat it as a runtime error
            // In the future, we could create a custom exception that carries the partial result
            PyRuntimeError::new_err(format!("Partial extraction: {}", message))
        }
    }
}
