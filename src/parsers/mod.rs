//! Parser implementations for various file formats
//!
//! This module contains the parser trait and implementations for different file formats.
//! Parsers are organized into categories:
//!
//! - **text**: Plain text, JSON, CSV, XML
//! - **document**: PDF, DOCX, ODT
//! - **image**: JPEG, PNG, TIFF
//! - **archive**: ZIP, TAR
//!
//! All parsers implement the `Parser` trait and are registered in the `ParserRegistry`.
//!
//! # Examples
//!
//! ## Using the parser registry
//!
//! ```
//! use omniparse::parsers::ParserRegistry;
//!
//! let registry = ParserRegistry::default();
//! let types = registry.supported_types();
//! println!("Supported formats: {}", types.len());
//!
//! if let Some(parser) = registry.get_parser("application/pdf") {
//!     println!("Found parser: {}", parser.name());
//! }
//! ```
//!
//! ## Implementing a custom parser
//!
//! ```
//! use omniparse::parsers::Parser;
//! use omniparse::core::{ExtractionResult, Result, Content, Metadata};
//!
//! struct MyCustomParser;
//!
//! impl Parser for MyCustomParser {
//!     fn supported_types(&self) -> &[&str] {
//!         &["application/x-custom"]
//!     }
//!
//!     fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
//!         // Custom parsing logic here
//!         Ok(ExtractionResult {
//!             mime_type: mime_type.to_string(),
//!             content: Content::Text("Custom content".to_string()),
//!             metadata: Metadata::new(),
//!             detection_confidence: 1.0,
//!         })
//!     }
//!
//!     fn name(&self) -> &str {
//!         "MyCustomParser"
//!     }
//! }
//! ```

use crate::core::{ExtractionResult, Result};
use std::collections::HashMap;
use std::io::Read;

/// Trait that all format-specific parsers must implement
///
/// This trait defines the interface for parsers that extract content and metadata
/// from specific file formats. Parsers must be thread-safe (`Send + Sync`) to
/// support parallel processing.
///
/// # Examples
///
/// See the module-level documentation for an example of implementing this trait.
pub trait Parser: Send + Sync {
    /// Returns the MIME types this parser handles
    ///
    /// A parser can support multiple MIME types if they share the same format
    /// (e.g., "text/plain" and "text/x-plain").
    fn supported_types(&self) -> &[&str];
    
    /// Extract content from the provided data
    ///
    /// This is the main parsing method that processes the file data and
    /// returns extracted content and metadata.
    ///
    /// # Arguments
    ///
    /// * `data` - The file data as a byte slice
    /// * `mime_type` - The MIME type of the data
    ///
    /// # Returns
    ///
    /// Returns an `ExtractionResult` with the extracted content and metadata.
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult>;
    
    /// Optional: streaming parse for large files
    ///
    /// This method allows parsers to process large files without loading them
    /// entirely into memory. The default implementation reads all data into
    /// memory and calls `parse()`, but parsers can override this for true
    /// streaming support.
    ///
    /// # Arguments
    ///
    /// * `reader` - A reader providing the file data
    /// * `mime_type` - The MIME type of the data
    fn parse_stream(&self, reader: &mut dyn Read, mime_type: &str) -> Result<ExtractionResult> {
        // Default implementation reads all into memory
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        self.parse(&buffer, mime_type)
    }
    
    /// Parser name for debugging
    ///
    /// Returns a human-readable name for this parser, used in error messages
    /// and logging.
    fn name(&self) -> &str;
}

/// Registry for managing all available parsers
///
/// The `ParserRegistry` maintains a collection of parsers and provides
/// efficient lookup by MIME type. The default registry includes all
/// built-in parsers.
///
/// # Examples
///
/// ```
/// use omniparse::parsers::ParserRegistry;
///
/// // Create registry with all built-in parsers
/// let registry = ParserRegistry::default();
///
/// // Check supported types
/// let types = registry.supported_types();
/// println!("Supports {} formats", types.len());
///
/// // Get a specific parser
/// if let Some(parser) = registry.get_parser("application/pdf") {
///     println!("PDF parser: {}", parser.name());
/// }
/// ```
pub struct ParserRegistry {
    parsers: Vec<Box<dyn Parser>>,
    mime_to_parser: HashMap<String, usize>,
}

impl ParserRegistry {
    /// Create a new empty parser registry
    ///
    /// This creates an empty registry with no parsers. Use `register()` to
    /// add parsers, or use `ParserRegistry::default()` to get a registry
    /// with all built-in parsers.
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::parsers::ParserRegistry;
    ///
    /// let mut registry = ParserRegistry::new();
    /// assert_eq!(registry.supported_types().len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            parsers: Vec::new(),
            mime_to_parser: HashMap::new(),
        }
    }

    /// Register a new parser in the registry
    ///
    /// This adds a parser to the registry and maps all of its supported
    /// MIME types to it. If a MIME type is already registered, it will
    /// be overwritten.
    ///
    /// # Arguments
    ///
    /// * `parser` - The parser to register
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::parsers::{ParserRegistry, text::PlainTextParser};
    ///
    /// let mut registry = ParserRegistry::new();
    /// registry.register(Box::new(PlainTextParser));
    /// assert!(registry.get_parser("text/plain").is_some());
    /// ```
    pub fn register(&mut self, parser: Box<dyn Parser>) {
        let index = self.parsers.len();
        let supported = parser.supported_types();
        
        // Map each MIME type to this parser's index
        for mime_type in supported {
            self.mime_to_parser.insert(mime_type.to_string(), index);
        }
        
        self.parsers.push(parser);
    }

    /// Get a parser for the specified MIME type
    ///
    /// Returns `None` if no parser is registered for the given MIME type.
    ///
    /// # Arguments
    ///
    /// * `mime_type` - The MIME type to look up
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::parsers::ParserRegistry;
    ///
    /// let registry = ParserRegistry::default();
    ///
    /// if let Some(parser) = registry.get_parser("application/pdf") {
    ///     println!("Found: {}", parser.name());
    /// } else {
    ///     println!("PDF parser not available");
    /// }
    /// ```
    pub fn get_parser(&self, mime_type: &str) -> Option<&dyn Parser> {
        self.mime_to_parser
            .get(mime_type)
            .and_then(|&index| self.parsers.get(index))
            .map(|boxed| boxed.as_ref())
    }

    /// Get all supported MIME types
    ///
    /// Returns a vector of all MIME types that have registered parsers.
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::parsers::ParserRegistry;
    ///
    /// let registry = ParserRegistry::default();
    /// let types = registry.supported_types();
    ///
    /// println!("Supported formats:");
    /// for mime_type in types {
    ///     println!("  - {}", mime_type);
    /// }
    /// ```
    pub fn supported_types(&self) -> Vec<String> {
        self.mime_to_parser.keys().cloned().collect()
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        
        // Register Phase 1 text parsers
        registry.register(Box::new(text::PlainTextParser));
        registry.register(Box::new(text::JsonParser));
        registry.register(Box::new(text::CsvParser));
        registry.register(Box::new(text::XmlParser));
        registry.register(Box::new(text::HtmlParser));
        registry.register(Box::new(text::CssParser));
        registry.register(Box::new(text::RtfParser));
        
        // Register Phase 2 document parsers
        registry.register(Box::new(document::PdfParser));
        registry.register(Box::new(document::DocxParser));
        registry.register(Box::new(document::OdtParser));
        registry.register(Box::new(document::XlsxParser));
        registry.register(Box::new(document::PptxParser));
        registry.register(Box::new(document::OdsParser));
        registry.register(Box::new(document::OdpParser));
        registry.register(Box::new(document::XlsParser));
        registry.register(Box::new(document::DocParser));
        registry.register(Box::new(document::PptParser));
        
        // Register Phase 3 image parsers
        registry.register(Box::new(image::JpegParser));
        registry.register(Box::new(image::PngParser));
        registry.register(Box::new(image::TiffParser));
        
        // Register Phase 4 archive parsers
        registry.register(Box::new(archive::ZipParser));
        registry.register(Box::new(archive::TarParser));
        
        registry
    }
}

pub mod text;
pub mod document;
pub mod image;
pub mod archive;
