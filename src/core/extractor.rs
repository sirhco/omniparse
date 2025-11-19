//! Main extraction orchestrator
//!
//! This module contains the `Extractor` type, which coordinates type detection
//! and parser selection to extract content from files. It's the core component
//! that ties together the detection and parsing subsystems.
//!
//! # Examples
//!
//! ```no_run
//! use omniparse::core::Extractor;
//!
//! let extractor = Extractor::new();
//! let result = extractor.extract_from_path("document.pdf")?;
//! println!("Extracted from {}", result.mime_type);
//! # Ok::<(), omniparse::Error>(())
//! ```

use crate::core::{Error, ExtractionResult, Result};
use crate::detection::TypeDetector;
use crate::parsers::ParserRegistry;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Main orchestrator for content extraction
///
/// The `Extractor` coordinates the extraction process by:
/// 1. Detecting the file type using the `TypeDetector`
/// 2. Finding the appropriate parser from the `ParserRegistry`
/// 3. Invoking the parser to extract content and metadata
///
/// # Examples
///
/// ```no_run
/// use omniparse::core::Extractor;
///
/// let extractor = Extractor::new();
///
/// // Extract from a file path
/// let result = extractor.extract_from_path("document.pdf")?;
/// println!("Type: {}", result.mime_type);
///
/// // Extract from bytes
/// let data = std::fs::read("file.json")?;
/// let result = extractor.extract_from_bytes(&data, None)?;
/// # Ok::<(), omniparse::Error>(())
/// ```
pub struct Extractor {
    pub(crate) detector: TypeDetector,
    pub(crate) registry: ParserRegistry,
}

impl Extractor {
    /// Create a new Extractor with default detector and parser registry
    ///
    /// This creates an extractor with all built-in parsers registered and
    /// ready to use.
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::core::Extractor;
    ///
    /// let extractor = Extractor::new();
    /// ```
    pub fn new() -> Self {
        Self {
            detector: TypeDetector::new(),
            registry: ParserRegistry::default(),
        }
    }

    /// Extract content from a file path
    ///
    /// This method:
    /// 1. Detects the file type by analyzing the file content and path
    /// 2. Selects the appropriate parser for the detected type
    /// 3. Extracts content and metadata using the parser
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to extract from
    ///
    /// # Returns
    ///
    /// Returns an `ExtractionResult` with the detected MIME type, content,
    /// metadata, and detection confidence.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The file type cannot be detected
    /// - The detected format is not supported
    /// - Parsing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use omniparse::core::Extractor;
    ///
    /// let extractor = Extractor::new();
    /// let result = extractor.extract_from_path("document.pdf")?;
    ///
    /// println!("MIME type: {}", result.mime_type);
    /// println!("Confidence: {:.2}", result.detection_confidence);
    /// # Ok::<(), omniparse::Error>(())
    /// ```
    pub fn extract_from_path(&self, path: impl AsRef<Path>) -> Result<ExtractionResult> {
        let path = path.as_ref();
        
        // Detect the file type
        let detection = self.detector.detect_from_path(path)?;
        
        // Get the appropriate parser
        let parser = self.registry.get_parser(&detection.mime_type)
            .ok_or_else(|| Error::UnsupportedFormat(detection.mime_type.clone()))?;
        
        // Open the file and create a buffered reader
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        
        // Parse the file using the streaming interface
        let mut result = parser.parse_stream(&mut reader, &detection.mime_type)?;
        
        // Update the result with detection information
        result.detection_confidence = detection.confidence;
        result.mime_type = detection.mime_type;
        
        Ok(result)
    }

    /// Extract content from a byte slice with optional MIME type hint
    ///
    /// This method allows extraction from in-memory data. If a MIME type hint
    /// is provided, type detection is skipped and the specified parser is used
    /// directly. Otherwise, type detection is performed on the byte data.
    ///
    /// # Arguments
    ///
    /// * `data` - Byte slice containing the file data
    /// * `mime_hint` - Optional MIME type hint to skip detection
    ///
    /// # Returns
    ///
    /// Returns an `ExtractionResult` with the extracted content and metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The format is not supported
    /// - Parsing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use omniparse::core::Extractor;
    ///
    /// let extractor = Extractor::new();
    /// let data = std::fs::read("file.json")?;
    ///
    /// // With automatic detection
    /// let result = extractor.extract_from_bytes(&data, None)?;
    ///
    /// // With MIME type hint
    /// let result = extractor.extract_from_bytes(&data, Some("application/json"))?;
    /// # Ok::<(), omniparse::Error>(())
    /// ```
    pub fn extract_from_bytes(&self, data: &[u8], mime_hint: Option<&str>) -> Result<ExtractionResult> {
        // Determine the MIME type
        let mime_type = if let Some(hint) = mime_hint {
            hint.to_string()
        } else {
            let detection = self.detector.detect_from_bytes(data);
            detection.mime_type
        };
        
        // Get the appropriate parser
        let parser = self.registry.get_parser(&mime_type)
            .ok_or_else(|| Error::UnsupportedFormat(mime_type.clone()))?;
        
        // Parse the data
        let mut result = parser.parse(data, &mime_type)?;
        
        // Set the MIME type in the result
        result.mime_type = mime_type;
        
        Ok(result)
    }
}

impl Default for Extractor {
    fn default() -> Self {
        Self::new()
    }
}
