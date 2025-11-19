//! Core types and functionality for Omniparse
//!
//! This module provides the fundamental types used throughout the library,
//! including error handling, result types, and the main extractor.

pub mod error;
pub mod extractor;
pub mod result;

// Re-export commonly used types
pub use error::{Error, Result};
pub use extractor::Extractor;
pub use result::{Content, ExtractionResult, Metadata, MetadataValue};
